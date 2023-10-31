use anyhow::Result;
use indicatif::HumanDuration;
use std::{path::PathBuf, time::Duration};

use console::style;
use log::debug;
use tokio::time::{sleep, Instant};

use crate::{
    api::{RapiClient, RapiReqwestClient},
    artifacts::{download_artifacts, fetch_artifact_list},
};

pub struct DownloadArtifactsInteractor {}

impl DownloadArtifactsInteractor {
    pub(crate) async fn execute(
        &self,
        base_url: &str,
        api_key: &str,
        id: &str,
        wait: bool,
        output: &PathBuf,
    ) -> Result<()> {
        let started = Instant::now();
        println!("{} Checking test run state...", style("[1/4]").bold().dim());
        let client = RapiReqwestClient::new(base_url, api_key);
        let stat = client.get_run(id).await?;
        if stat.completed.is_none() && wait {
            loop {
                if stat.completed.is_some() {
                    break;
                }
                sleep(Duration::new(5, 0)).await;
            }
        } else {
            debug!("Test run {} finished", &id);
        }
        println!("{} Fetching file list...", style("[2/4]").bold().dim());
        let token = client.get_token().await?;
        let artifacts = fetch_artifact_list(&client, id, &token).await?;
        println!("{} Downloading files...", style("[3/4]").bold().dim());
        download_artifacts(&client, artifacts, output, &token, true).await?;
        println!(
            "{} Patching local relative paths...",
            style("[4/4]").bold().dim()
        );

        println!("Done in {}", HumanDuration(started.elapsed()));
        Ok(())
    }
}

pub struct TriggerTestRunInteractor {}

impl TriggerTestRunInteractor {
    pub(crate) async fn execute(
        &self,
        base_url: &str,
        api_key: &str,
        wait: bool,
        isolated: Option<bool>,
        output: &Option<PathBuf>,
        application: Option<PathBuf>,
        test_application: PathBuf,
        os_version: Option<String>,
        system_image: Option<String>,
        platform: String,
    ) -> Result<()> {
        let client = RapiReqwestClient::new(base_url, api_key);
        let id = client
            .create_run(
                application,
                test_application,
                None,
                None,
                platform,
                os_version,
                system_image,
                isolated,
            )
            .await?;

        if wait {
            loop {
                let stat = client.get_run(&id).await?;
                if stat.completed.is_some() {
                    println!("Report - {}/report/{}", base_url, id);
                    println!(
                        "Passed - {}",
                        stat.passed
                            .map(|x| x.to_string())
                            .unwrap_or("missing".to_owned())
                    );
                    println!(
                        "Failed - {}",
                        stat.failed
                            .map(|x| x.to_string())
                            .unwrap_or("missing".to_owned())
                    );
                    println!(
                        "Ignored - {}",
                        stat.ignored
                            .map(|x| x.to_string())
                            .unwrap_or("missing".to_owned())
                    );

                    if let Some(output) = output {
                        let token = client.get_token().await?;
                        let artifacts = fetch_artifact_list(&client, &id, &token).await?;
                        download_artifacts(&client, artifacts, output, &token, false).await?;
                    }
                    return Ok(());
                }
                sleep(Duration::new(5, 0)).await;
            }
        } else {
            println!("Test run {} started", id);
            Ok(())
        }
    }
}
