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
                    let result = async {
                        let mut dst = fs::File::create(&local_path).await?;
                        bucket
                            .get_object_to_writer(&s3_key, &mut dst)
                            .await
                            .map_err(|e| anyhow::anyhow!("{}", e))
                    }
                    .await;
                    match result {
                        Ok(_) => {
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
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tempfile::tempdir;

    fn link(key: &str, target: &str) -> ManifestLink {
        ManifestLink {
            key: key.to_string(),
            target: target.to_string(),
        }
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
