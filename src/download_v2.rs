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

use crate::api::{DownloadConfig, Manifest, ManifestFile, ManifestLink};

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
    let response = bucket.get_object(&manifest_key).await
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

    let download_results: Vec<Result<()>> = stream::iter(files.into_iter().map(|file| {
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
                    bucket.get_object_to_writer(&s3_key, &mut dst).await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }.await;
                match result {
                    Ok(_) => {
                        if let Some(pb) = &pb {
                            pb.inc(1);
                        }
                        return Ok(());
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

    for result in &download_results {
        if let Err(e) = result {
            return Err(anyhow::anyhow!("Download failed: {}", e));
        }
    }

    for link in &links {
        let target_path = output.join(&link.target);
        let link_path = output.join(&link.key);
        if target_path.exists() {
            create_link(&target_path, &link_path).await?;
            if let Some(pb) = &progress_bar {
                pb.inc(1);
            }
        } else {
            log::warn!(
                "Link target not found, skipping: {} -> {}",
                link.key, link.target
            );
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_with_message("done");
    }

    Ok(())
}
