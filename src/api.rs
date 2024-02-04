use std::{
    cmp::min,
    path::{Path, PathBuf}, time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{multipart::Part, Body, Client, StatusCode};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::{
    fs::{create_dir_all, File},
    io,
};
use tokio_util::io::ReaderStream;

use crate::{
    errors::{ApiError, InputError},
    filtering::SparseMarathonfile,
};

#[async_trait]
pub trait RapiClient {
    async fn get_token(&self) -> Result<String>;
    async fn create_run(
        &self,
        app: Option<PathBuf>,
        test_app: PathBuf,
        name: Option<String>,
        link: Option<String>,
        platform: String,
        os_version: Option<String>,
        system_image: Option<String>,
        isolated: Option<bool>,
        filtering_configuration: Option<SparseMarathonfile>,
        progress: bool,
    ) -> Result<String>;
    async fn get_run(&self, id: &str) -> Result<TestRun>;

    async fn list_artifact(&self, jwt_token: &str, id: &str) -> Result<Vec<Artifact>>;
    async fn download_artifact(
        &self,
        jwt_token: &str,
        artifact: Artifact,
        base_path: PathBuf,
    ) -> Result<()>;
}

#[derive(Clone)]
pub struct RapiReqwestClient {
    base_url: String,
    api_key: String,
    client: Client,
}

impl RapiReqwestClient {
    pub fn new(base_url: &str, api_key: &str) -> RapiReqwestClient {
        let non_sanitized = base_url.to_string();
        RapiReqwestClient {
            base_url: non_sanitized
                .strip_suffix('/')
                .unwrap_or(&non_sanitized)
                .to_string(),
            api_key: api_key.to_string(),
            ..Default::default()
        }
    }
}

impl Default for RapiReqwestClient {
    fn default() -> Self {
        Self {
            base_url: String::from("https:://cloud.marathonlabs.io/api/v1"),
            api_key: "".into(),
            client: Client::default(),
        }
    }
}

#[async_trait]
impl RapiClient for RapiReqwestClient {
    async fn get_token(&self) -> Result<String> {
        let url = format!("{}/user/jwt", self.base_url);
        let params = [("api_key", self.api_key.clone())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(api_error_adapter)?
            .json::<GetTokenResponse>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;
        Ok(response.token)
    }

    async fn create_run(
        &self,
        app: Option<PathBuf>,
        test_app: PathBuf,
        name: Option<String>,
        link: Option<String>,
        platform: String,
        os_version: Option<String>,
        system_image: Option<String>,
        isolated: Option<bool>,
        filtering_configuration: Option<SparseMarathonfile>,
        progress: bool,
    ) -> Result<String> {
        let url = format!("{}/run", self.base_url);
        let params = [("api_key", self.api_key.clone())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;

        let mut form = reqwest::multipart::Form::new().text("platform", platform);

        let file = File::open(&test_app)
            .await
            .map_err(|error| InputError::OpenFileFailure {
                path: test_app.clone(),
                error,
            })?;
        let test_app_file_name = test_app
            .file_name()
            .map(|val| val.to_string_lossy().to_string())
            .ok_or(InputError::InvalidFileName {
                path: test_app.clone(),
            })?;
        let test_app_total_size = (&file).metadata().await?.len();
        let mut test_app_reader = ReaderStream::new(file);
        let mut multi_progress: Option<MultiProgress> = if progress {
            Some(MultiProgress::new())
        } else {
            None
        };
        let test_app_progress_bar;
        let test_app_body;
        if progress {
            let sty = ProgressStyle::with_template(
                "{spinner} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
            )
            .unwrap()
            .progress_chars("#>-");

            let pb = ProgressBar::new(test_app_total_size);
            pb.enable_steady_tick(Duration::from_millis(120));
            test_app_progress_bar = multi_progress
                .as_mut()
                .unwrap()
                .add(pb);
            test_app_progress_bar.set_style(sty.clone());
            let mut test_app_progress = 0u64;
            let test_app_stream = async_stream::stream! {
                while let Some(chunk) = test_app_reader.next().await {
                    let test_app_progress_bar = test_app_progress_bar.clone();
                    if let Ok(chunk) = &chunk {
                        let new = min(test_app_progress + (chunk.len() as u64), test_app_total_size);
                        test_app_progress = new;
                        test_app_progress_bar.set_position(new);
                        if test_app_progress >= test_app_total_size {
                            test_app_progress_bar.finish_and_clear();
                        }
                    }
                    yield chunk;
                }
            };
            test_app_body = Body::wrap_stream(test_app_stream);
        } else {
            test_app_body = Body::wrap_stream(test_app_reader);
        }
        form = form.part(
            "testapp",
            Part::stream_with_length(test_app_body, test_app_total_size)
                .file_name(test_app_file_name),
        );

        if let Some(app) = app {
            let file = File::open(&app)
                .await
                .map_err(|error| InputError::OpenFileFailure {
                    path: app.clone(),
                    error,
                })?;

            let app_file_name = app
                .file_name()
                .map(|val| val.to_string_lossy().to_string())
                .ok_or(InputError::InvalidFileName { path: app.clone() })?;

            let app_total_size = (&file).metadata().await?.len();
            let mut app_reader = ReaderStream::new(file);
            let app_body;

            if progress {
                let pb = ProgressBar::new(app_total_size);
                pb.enable_steady_tick(Duration::from_millis(120));
                let app_progress_bar = multi_progress
                    .unwrap()
                    .add(pb);
                let sty = ProgressStyle::with_template(
                    "{spinner} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
                )
                .unwrap()
                .progress_chars("#>-");
                app_progress_bar.set_style(sty);

                let mut app_progress = 0u64;
                let app_stream = async_stream::stream! {
                    while let Some(chunk) = app_reader.next().await {
                        let app_progress_bar = app_progress_bar.clone();
                        if let Ok(chunk) = &chunk {
                            let new = min(app_progress + (chunk.len() as u64), app_total_size);
                            app_progress = new;
                            app_progress_bar.set_position(new);
                            if app_progress >= app_total_size {
                                app_progress_bar.finish_and_clear();
                            }
                        }
                        yield chunk;
                    }
                };
                app_body = Body::wrap_stream(app_stream);
            } else {
                app_body = Body::wrap_stream(app_reader);
            }

            form = form.part(
                "app",
                Part::stream_with_length(app_body, app_total_size).file_name(app_file_name),
            );
        }

        if let Some(name) = name {
            form = form.text("name", name)
        }

        if let Some(link) = link {
            form = form.text("link", link)
        }

        if let Some(os_version) = os_version {
            form = form.text("osversion", os_version)
        }

        if let Some(system_image) = system_image {
            form = form.text("system_image", system_image)
        }

        if let Some(isolated) = isolated {
            form = form.text("isolated", isolated.to_string())
        }

        if let Some(filtering_configuration) = filtering_configuration {
            form = form.text(
                "filtering_configuration",
                serde_json::to_string(&filtering_configuration)?,
            );
        }

        let response = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(api_error_adapter)?
            .json::<CreateRunResponse>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;

        Ok(response.run_id)
    }

    async fn get_run(&self, id: &str) -> Result<TestRun> {
        let url = format!("{}/run/{}", self.base_url, id);
        let params = [("api_key", self.api_key.clone())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(api_error_adapter)?
            .json::<TestRun>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;
        Ok(response)
    }

    async fn list_artifact(&self, jwt_token: &str, id: &str) -> Result<Vec<Artifact>> {
        let url = format!("{}/artifact/{}", self.base_url, id);

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await
            .map_err(api_error_adapter)?
            .json::<Vec<Artifact>>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;

        Ok(response)
    }

    async fn download_artifact(
        &self,
        jwt_token: &str,
        artifact: Artifact,
        base_path: PathBuf,
    ) -> Result<()> {
        let url = format!("{}/artifact", self.base_url);
        let params = [("key", artifact.id.to_owned())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;

        let relative_path = artifact.id.strip_prefix('/').unwrap_or(&artifact.id);
        let relative_path = Path::new(&relative_path);
        let mut absolute_path = base_path.clone();
        absolute_path.push(relative_path);

        let mut src = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await
            .map_err(api_error_adapter)?
            .bytes_stream();

        let dst_dir = absolute_path.parent();
        if let Some(dst_dir) = dst_dir {
            if !dst_dir.is_dir() {
                create_dir_all(dst_dir).await?;
            }
        }
        let mut dst = File::create(absolute_path).await?;

        while let Some(chunk) = src.next().await {
            io::copy(&mut chunk?.as_ref(), &mut dst).await?;
        }

        Ok(())
    }
}

fn api_error_adapter(error: reqwest::Error) -> ApiError {
    if let Some(status) = error.status() {
        match status {
            StatusCode::UNAUTHORIZED => ApiError::Unauthorized { error },
            _ => ApiError::RequestFailed { error },
        }
    } else {
        ApiError::RequestFailed { error }
    }
}

#[derive(Deserialize)]
pub struct CreateRunResponse {
    #[serde(rename = "run_id")]
    pub run_id: String,
    #[serde(rename = "status")]
    pub status: String,
}

#[derive(Deserialize)]
pub struct TestRun {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "passed")]
    pub passed: Option<u32>,
    #[serde(rename = "failed")]
    pub failed: Option<u32>,
    #[serde(rename = "ignored")]
    pub ignored: Option<u32>,
    #[serde(rename = "completed", with = "time::serde::iso8601::option")]
    pub completed: Option<OffsetDateTime>,
}

#[derive(Deserialize)]
pub struct GetTokenResponse {
    #[serde(rename = "token")]
    pub token: String,
}

#[derive(Deserialize, Clone)]
pub struct Artifact {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "is_file")]
    pub is_file: bool,
}
