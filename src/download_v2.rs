use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use futures::{stream, StreamExt};
use indicatif::ProgressBar;
use log::debug;
use s3::creds::Credentials;
use s3::Bucket;
use s3::Region;
use tokio::fs::{self, create_dir_all};

use crate::api::{decompress_gz_in_place, DownloadConfig, Manifest, ManifestFile, ManifestLink};

#[cfg(unix)]
async fn create_link(target: &Path, link: &Path) -> Result<()> {
    let relative = pathdiff::diff_paths(target, link.parent().unwrap_or(Path::new("")))
        .unwrap_or_else(|| target.to_path_buf());
    tokio::fs::symlink(&relative, link).await?;
    Ok(())
}

#[cfg(not(unix))]
async fn create_link(target: &Path, link: &Path) -> Result<()> {
    fs::copy(target, link).await?;
    Ok(())
}

const DOWNLOAD_CONCURRENCY: usize = 64;

/// Decompresses a downloaded `*.gz` file next to itself (`foo.log.gz` -> `foo.log`).
/// Returns the original key when decompression happened. On failure the `.gz` file is kept.
async fn decompress_if_gz(key: &str, local_path: &Path) -> Option<String> {
    key.strip_suffix(".gz")?;
    let path = local_path.to_path_buf();
    let result = tokio::task::spawn_blocking(move || decompress_gz_in_place(&path))
        .await
        .map_err(anyhow::Error::from)
        .and_then(|r| r);
    match result {
        Ok(()) => Some(key.to_string()),
        Err(e) => {
            log::warn!("Failed to decompress {}, keeping it as is: {}", key, e);
            let _ = fs::remove_file(local_path.with_extension("")).await;
            None
        }
    }
}

/// Rewrites a link whose target was decompressed so it points at the decompressed file,
/// dropping the `.gz` suffix from the link name as well.
fn resolve_link<'a>(link: &'a ManifestLink, decompressed: &HashSet<String>) -> (&'a str, &'a str) {
    match link.target.strip_suffix(".gz") {
        Some(target) if decompressed.contains(&link.target) => {
            let key = link.key.strip_suffix(".gz").unwrap_or(&link.key);
            (key, target)
        }
        _ => (&link.key, &link.target),
    }
}

fn create_bucket(config: &DownloadConfig) -> Result<Box<Bucket>> {
    let region = Region::Custom {
        region: config.region.clone(),
        endpoint: config.endpoint.clone(),
    };
    let credentials = Credentials::new(
        Some(&config.credentials.access_key_id),
        Some(&config.credentials.secret_access_key),
        Some(&config.credentials.session_token),
        None,
        None,
    )?;
    Ok(Bucket::new(&config.bucket, region, credentials)?.with_path_style())
}

pub async fn fetch_manifest(config: &DownloadConfig) -> Result<Manifest> {
    let bucket = create_bucket(config)?;
    let manifest_key = format!("{}manifest.json", config.prefix);
    let response = bucket
        .get_object(&manifest_key)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch manifest from R2: {}", e))?;
    let manifest: Manifest = serde_json::from_slice(response.bytes())
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest: {}", e))?;
    Ok(manifest)
}

pub async fn download_via_manifest(
    config: &DownloadConfig,
    output: &Path,
    files: Vec<ManifestFile>,
    links: Vec<ManifestLink>,
    no_progress_bar: bool,
) -> Result<()> {
    let bucket = create_bucket(config)?;

    let progress_bar = if !no_progress_bar {
        Some(ProgressBar::new((files.len() + links.len()) as u64))
    } else {
        None
    };

    let mut dirs: HashSet<String> = HashSet::new();
    for f in &files {
        if let Some(parent) = Path::new(&f.key).parent() {
            let p = parent.to_string_lossy().to_string();
            if !p.is_empty() {
                dirs.insert(p);
            }
        }
    }
    for l in &links {
        if let Some(parent) = Path::new(&l.key).parent() {
            let p = parent.to_string_lossy().to_string();
            if !p.is_empty() {
                dirs.insert(p);
            }
        }
    }
    for dir in &dirs {
        let full = output.join(dir);
        create_dir_all(&full).await?;
    }

    let download_results: Vec<Result<Option<String>>> =
        stream::iter(files.into_iter().map(|file| {
            let bucket = bucket.clone();
            let prefix = config.prefix.clone();
            let output = output.to_path_buf();
            let pb = progress_bar.clone();
            async move {
                let s3_key = format!("{}{}", prefix, file.key);
                let local_path = output.join(&file.key);

                let mut last_err = None;
                for attempt in 1..=3 {
                    let result: Result<()> = async {
                        let mut dst = fs::File::create(&local_path).await?;
                        let status = bucket
                            .get_object_to_writer(&s3_key, &mut dst)
                            .await
                            .map_err(|e| anyhow::anyhow!("{}", e))?;
                        if !(200..300).contains(&status) {
                            anyhow::bail!("HTTP status {}", status);
                        }

                        tokio::io::AsyncWriteExt::flush(&mut dst).await?;
                        let downloaded_size = fs::metadata(&local_path).await?.len();
                        if downloaded_size != file.size {
                            anyhow::bail!("expected {} bytes, got {}", file.size, downloaded_size);
                        }

                        Ok(())
                    }
                    .await;
                    match result {
                        Ok(()) => {
                            let decompressed = decompress_if_gz(&file.key, &local_path).await;
                            if let Some(pb) = &pb {
                                pb.inc(1);
                            }
                            return Ok(decompressed);
                        }
                        Err(e) => {
                            debug!("Attempt {}/3 failed for {}: {}", attempt, file.key, e);
                            last_err = Some(e);
                        }
                    }
                }
                Err(anyhow::anyhow!(
                    "Failed to download {} after 3 attempts: {}",
                    file.key,
                    last_err.unwrap()
                ))
            }
        }))
        .buffer_unordered(DOWNLOAD_CONCURRENCY)
        .collect()
        .await;

    let mut decompressed: HashSet<String> = HashSet::new();
    for result in download_results {
        match result {
            Ok(Some(key)) => {
                decompressed.insert(key);
            }
            Ok(None) => {}
            Err(e) => return Err(anyhow::anyhow!("Download failed: {}", e)),
        }
    }

    for link in &links {
        let (link_key, link_target) = resolve_link(link, &decompressed);
        let target_path = output.join(link_target);
        let link_path = output.join(link_key);
        if target_path.exists() {
            create_link(&target_path, &link_path).await?;
            if let Some(pb) = &progress_bar {
                pb.inc(1);
            }
        } else {
            log::warn!(
                "Link target not found, skipping: {} -> {}",
                link.key,
                link.target
            );
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_with_message("done");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::TempCredentials;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    fn link(key: &str, target: &str) -> ManifestLink {
        ManifestLink {
            key: key.to_string(),
            target: target.to_string(),
        }
    }

    fn file(key: &str, size: u64) -> ManifestFile {
        ManifestFile {
            key: key.to_string(),
            size,
        }
    }

    fn download_config(endpoint: String) -> DownloadConfig {
        DownloadConfig {
            credentials: TempCredentials {
                access_key_id: "access-key".to_string(),
                secret_access_key: "secret-key".to_string(),
                session_token: "session-token".to_string(),
                expiration: "2100-01-01T00:00:00Z".to_string(),
            },
            bucket: "test-bucket".to_string(),
            endpoint,
            region: "us-east-1".to_string(),
            prefix: "run-id/".to_string(),
        }
    }

    async fn serve_response(status: &'static str, body: &'static [u8]) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 4096];
                let _ = socket.read(&mut request).await;
                let headers = format!(
                    "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    status,
                    body.len()
                );
                socket.write_all(headers.as_bytes()).await.unwrap();
                socket.write_all(body).await.unwrap();
            }
        });

        (format!("http://{}", address), server)
    }

    #[tokio::test]
    async fn decompresses_gz_file_in_place() {
        let dir = tempdir().unwrap();
        let gz_path = dir.path().join("test.log.gz");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"hello log").unwrap();
        std::fs::write(&gz_path, encoder.finish().unwrap()).unwrap();

        let result = decompress_if_gz("logs/test.log.gz", &gz_path).await;

        assert_eq!(result.as_deref(), Some("logs/test.log.gz"));
        assert!(!gz_path.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.log")).unwrap(),
            "hello log"
        );
    }

    #[tokio::test]
    async fn keeps_invalid_gz_file() {
        let dir = tempdir().unwrap();
        let gz_path = dir.path().join("broken.log.gz");
        std::fs::write(&gz_path, b"not gzip").unwrap();

        let result = decompress_if_gz("broken.log.gz", &gz_path).await;

        assert_eq!(result, None);
        assert!(gz_path.exists());
        assert!(!dir.path().join("broken.log").exists());
    }

    #[tokio::test]
    async fn skips_non_gz_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("video.mp4");
        std::fs::write(&path, b"data").unwrap();

        assert_eq!(decompress_if_gz("video.mp4", &path).await, None);
        assert_eq!(std::fs::read(&path).unwrap(), b"data");
    }

    #[tokio::test]
    async fn rejects_empty_error_response() {
        let (endpoint, server) = serve_response("404 Not Found", b"").await;
        let dir = tempdir().unwrap();
        let key = "report/allure-results/result.json";

        let result = download_via_manifest(
            &download_config(endpoint),
            dir.path(),
            vec![file(key, 0)],
            vec![],
            true,
        )
        .await;
        server.abort();

        let error = result.expect_err("an empty 404 response must not be a successful download");
        assert!(error.to_string().contains("404"), "{error}");
    }

    #[tokio::test]
    async fn rejects_response_with_unexpected_size() {
        let (endpoint, server) = serve_response("200 OK", b"{}").await;
        let dir = tempdir().unwrap();
        let key = "report/allure-results/result.json";

        let result = download_via_manifest(
            &download_config(endpoint),
            dir.path(),
            vec![file(key, 3)],
            vec![],
            true,
        )
        .await;
        server.abort();

        let error = result.expect_err("a short response must not be a successful download");
        assert!(
            error.to_string().contains("expected 3 bytes, got 2"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn downloads_response_with_expected_size() {
        let (endpoint, server) = serve_response("200 OK", b"{}").await;
        let dir = tempdir().unwrap();
        let key = "report/allure-results/result.json";

        download_via_manifest(
            &download_config(endpoint),
            dir.path(),
            vec![file(key, 2)],
            vec![],
            true,
        )
        .await
        .unwrap();
        server.abort();

        assert_eq!(std::fs::read(dir.path().join(key)).unwrap(), b"{}");
    }

    #[test]
    fn rewrites_link_to_decompressed_target() {
        let decompressed = HashSet::from(["logs/omni/a.log.gz".to_string()]);
        let l = link("report/data/attachments/abc.gz", "logs/omni/a.log.gz");

        assert_eq!(
            resolve_link(&l, &decompressed),
            ("report/data/attachments/abc", "logs/omni/a.log")
        );
    }

    #[test]
    fn keeps_link_when_target_not_decompressed() {
        let decompressed = HashSet::new();
        let l = link("report/data/attachments/abc.gz", "logs/omni/a.log.gz");

        assert_eq!(
            resolve_link(&l, &decompressed),
            ("report/data/attachments/abc.gz", "logs/omni/a.log.gz")
        );
    }
}
