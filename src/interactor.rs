use anyhow::Result;
use globset::Glob;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use std::{path::PathBuf, time::Duration};
use url::{Position, Url};

use log::debug;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    time::{sleep, Instant},
};

use crate::{
    api::{Artifact, RapiClient, RapiReqwestClient},
    artifacts::{download_artifacts, fetch_artifact_list},
    cli::{Platform, ResultFileFormat},
    filtering::model::SparseMarathonfile,
    formatter::{Formatter, StandardFormatter},
    progress::{TestRunFinished, TestRunStarted},
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
        glob: Option<String>,
        no_progress_bars: bool,
    ) -> Result<()> {
        let started = Instant::now();
        let formatter = StandardFormatter::new(4);
        formatter.stage("Checking test run state...");

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

        formatter.stage("Fetching file list...");
        let token = client.get_token().await?;
        let artifacts = fetch_artifact_list(&client, id, &token).await?;
        let test_run_id_prefix = format!("{}/", id);
        let artifacts = filter_artifact_list(artifacts, glob, &test_run_id_prefix)?;

        formatter.stage("Downloading files...");
        download_artifacts(&client, id, artifacts, output, &token, no_progress_bars).await?;
        formatter.stage("Patching local relative paths...");

        formatter.message(&format!("Done in {}", HumanDuration(started.elapsed())));
        Ok(())
    }
}

fn filter_artifact_list(
    artifacts: Vec<Artifact>,
    glob: Option<String>,
    prefix: &str,
) -> Result<Vec<crate::api::Artifact>> {
    match glob {
        Some(glob) => {
            let matcher = Glob::new(&glob)?.compile_matcher();
            Ok(artifacts
                .into_iter()
                .filter(|x| -> bool {
                    let predicate_result =
                        matcher.is_match(x.id.strip_prefix(prefix).unwrap_or(&x.id));
                    if !predicate_result {
                        debug!("Filtered out download of {}", &x.id);
                    }
                    predicate_result
                })
                .collect())
        }
        None => Ok(artifacts),
    }
}

pub struct TriggerTestRunInteractor {}

impl TriggerTestRunInteractor {
    pub(crate) async fn execute(
        &self,
        base_url: &str,
        api_key: &str,
        name: Option<String>,
        link: Option<String>,
        wait: bool,
        isolated: Option<bool>,
        ignore_test_failures: Option<bool>,
        code_coverage: Option<bool>,
        retry_quota_test_uncompleted: Option<u32>,
        retry_quota_test_preventive: Option<u32>,
        retry_quota_test_reactive: Option<u32>,
        analytics_read_only: Option<bool>,
        filtering_configuration: Option<SparseMarathonfile>,
        output: &Option<PathBuf>,
        application: Option<PathBuf>,
        test_application: PathBuf,
        os_version: Option<String>,
        system_image: Option<String>,
        device: Option<String>,
        xcode_version: Option<String>,
        flavor: Option<String>,
        platform: String,
        no_progress_bars: bool,
        result_file: Option<PathBuf>,
        result_file_format: ResultFileFormat,
        env_args: Option<Vec<String>>,
        test_env_args: Option<Vec<String>>,
    ) -> Result<bool> {
        let client = RapiReqwestClient::new(base_url, api_key);
        let steps = match (wait, output) {
            (true, Some(_)) => 5,
            (true, None) => 2,
            _ => 1,
        };
        let formatter = StandardFormatter::new(steps);

        let token = client.get_token().await?;

        formatter.stage("Submitting new run...");
        let id = client
            .create_run(
                application,
                test_application,
                name,
                link,
                platform,
                os_version,
                system_image,
                device,
                xcode_version,
                isolated,
                code_coverage,
                retry_quota_test_uncompleted,
                retry_quota_test_preventive,
                retry_quota_test_reactive,
                analytics_read_only,
                filtering_configuration,
                no_progress_bars,
                flavor,
                env_args,
                test_env_args,
            )
            .await?;

        if wait {
            formatter.stage("Waiting for test run to finish...");

            let spinner = if !no_progress_bars {
                let pb = ProgressBar::new_spinner();
                pb.enable_steady_tick(Duration::from_millis(80));
                pb.set_style(
                    ProgressStyle::with_template("{spinner:.blue}")
                        .unwrap()
                        .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]),
                );
                Some(pb)
            } else {
                None
            };
            loop {
                let stat = client.get_run(&id).await?;
                if stat.completed.is_some() {
                    if let Some(s) = spinner {
                        s.finish_and_clear()
                    }

                    let base_report_url = Url::parse(base_url)?;
                    let base_report_url = &base_report_url[..Position::AfterPort];

                    let state = stat.state.clone();
                    let report = format!("{}/report/{}", base_report_url, id);
                    let passed = stat.passed;
                    let failed = stat.failed;
                    let ignored = stat.ignored;

                    let event = TestRunFinished {
                        id: id.clone(),
                        state,
                        report,
                        passed,
                        failed,
                        ignored,
                    };
                    formatter.message(&format!("{}", event));
                    if let Some(result_file) = result_file {
                        let mut file = File::create(result_file).await?;
                        let data = match result_file_format {
                            ResultFileFormat::Json => serde_json::to_string(&event)?,
                            ResultFileFormat::Yaml => serde_yaml::to_string(&event)?,
                        };
                        file.write(data.as_bytes()).await?;
                    }

                    if let Some(output) = output {
                        formatter.stage("Fetching file list...");
                        let artifacts = fetch_artifact_list(&client, &id, &token).await?;
                        formatter.stage("Downloading files...");
                        download_artifacts(
                            &client,
                            &id,
                            artifacts,
                            output,
                            &token,
                            no_progress_bars,
                        )
                        .await?;
                        formatter.stage("Patching local relative paths...");
                    }
                    return match (stat.state.as_str(), ignore_test_failures) {
                        ("failure", Some(false) | None) => Ok(false),
                        (_, _) => Ok(true),
                    };
                }
                sleep(Duration::new(5, 0)).await;
            }
        } else {
            let event = TestRunStarted { id };
            formatter.message(&format!("{}", event));
            if let Some(result_file) = result_file {
                let mut file = File::create(result_file).await?;
                let data = match result_file_format {
                    ResultFileFormat::Json => serde_json::to_string(&event)?,
                    ResultFileFormat::Yaml => serde_yaml::to_string(&event)?,
                };
                file.write(data.as_bytes()).await?;
            }

            Ok(true)
        }
    }
}

pub struct GetDeviceCatalogInteractor {}

impl GetDeviceCatalogInteractor {
    pub(crate) async fn execute(
        &self,
        base_url: &str,
        api_key: &str,
        platform: &Platform,
        no_progress_bar: bool,
    ) -> Result<()> {
        let formatter = StandardFormatter::new(1);

        let mut progress_bar: Option<ProgressBar> = None;
        if !no_progress_bar {
            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(80));
            pb.set_style(
                ProgressStyle::with_template("{spinner:.blue} {msg}")?
                    .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]),
            );
            pb.set_message("Fetching device catalog...");
            progress_bar = Some(pb);
        } else {
            formatter.message("Fetching device catalog...");
        }
        let client = RapiReqwestClient::new(base_url, api_key);

        let token = client.get_token().await?;
        let devices = match platform {
            Platform::Android => client.get_devices_android(&token).await?,
            Platform::iOS => todo!(),
        };
        if let Some(progress_bar) = progress_bar {
            progress_bar.finish_and_clear();
        }
        println!("{}", serde_yaml::to_string(&devices)?);
        Ok(())
    }
}
