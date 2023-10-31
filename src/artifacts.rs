use std::path::PathBuf;

use ::futures::{stream, StreamExt, TryStreamExt};
use anyhow::anyhow;
use anyhow::Result;
use indicatif::ProgressBar;
use log::debug;

use crate::api::{Artifact, RapiClient, RapiReqwestClient};

pub async fn fetch_artifact_list(
    client: &RapiReqwestClient,
    id: &str,
    token: &str,
) -> Result<Vec<Artifact>> {
    let mut artifacts: Vec<Artifact> = Vec::new();
    let mut list: Vec<String> = vec![id.to_owned()];

    loop {
        let stats: Vec<Artifact> = stream::iter(list.clone().into_iter())
            .map(|dir| {
                let client = client.clone();
                let token = token.to_owned();
                tokio::spawn(async move { client.list_artifact(&token, &dir).await.unwrap() })
            })
            .buffer_unordered(num_cpus::get())
            .try_concat()
            .await
            .map_err(|_| anyhow!("Failed to retrieve artifact list"))?;

        list.clear();
        for f in stats {
            if f.is_file {
                artifacts.push(f);
            } else {
                list.push(f.id);
            }
        }

        if list.is_empty() {
            break;
        }
    }

    Ok(artifacts)
}

pub async fn download_artifacts(
    client: &RapiReqwestClient,
    artifacts: Vec<Artifact>,
    path: &PathBuf,
    token: &str,
    progress: bool,
) -> Result<()> {
    debug!("Downloading {} artifacts:", artifacts.len());

    artifacts.iter().for_each(|f| debug!("{}", f.id));

    let mut progress_bar: Option<ProgressBar> = None;
    if progress {
        progress_bar = Some(ProgressBar::new(artifacts.len() as u64))
    }

    stream::iter(artifacts.into_iter())
        .map(|artifact| {
            let client = client.clone();
            let token = token.to_owned();
            let base_path = path.clone();
            let progress_bar = progress_bar.clone();
            tokio::spawn(async move {
                client
                    .download_artifact(&token, artifact, base_path)
                    .await
                    .unwrap();

                if let Some(progress_bar) = progress_bar {
                    progress_bar.inc(1);
                }
            })
        })
        .buffer_unordered(num_cpus::get())
        .try_collect()
        .await
        .map_err(|_| anyhow!("Failed to retrieve artifacts"))?;

    if let Some(progress_bar) = progress_bar {
        progress_bar.finish_with_message("done");
    }
    Ok(())
}

