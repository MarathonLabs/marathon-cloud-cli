use std::{
    cmp::min,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{Body, Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::fs::{create_dir_all, File};
use tokio::io;

use crate::{
    bundle::ApplicationBundle,
    errors::{ApiError, EnvArgError, InputError},
    filtering::model::SparseMarathonfile,
    pull::PullFileConfig,
};

use tokio_util::io::ReaderStream;

#[async_trait]
pub trait RapiClient {
    async fn get_token(&self) -> Result<String>;
    async fn create_run(
        &self,
        app: Option<PathBuf>,
        test_app: Option<PathBuf>,
        name: Option<String>,
        link: Option<String>,
        branch: Option<String>,
        platform: String,
        os_version: Option<String>,
        system_image: Option<String>,
        device: Option<String>,
        xcode_version: Option<String>,
        isolated: Option<bool>,
        collect_code_coverage: Option<bool>,
        retry_quota_test_uncompleted: Option<u32>,
        retry_quota_test_preventive: Option<u32>,
        retry_quota_test_reactive: Option<u32>,
        profiling: Option<bool>,
        analytics_read_only: Option<bool>,
        filtering_configuration: Option<SparseMarathonfile>,
        no_progress_bar: bool,
        flavor: Option<String>,
        env_args: Option<Vec<String>>,
        test_env_args: Option<Vec<String>>,
        pull_file_config: Option<PullFileConfig>,
        concurrency_limit: Option<u32>,
        test_timeout_default: Option<u32>,
        test_timeout_max: Option<u32>,
        project: Option<String>,
        application_bundle: Option<Vec<ApplicationBundle>>,
        library_bundle: Option<Vec<PathBuf>>,
    ) -> Result<String>;
    async fn get_run(&self, id: &str) -> Result<TestRun>;

    async fn list_artifact(&self, jwt_token: &str, id: &str) -> Result<Vec<Artifact>>;
    async fn download_artifact(
        &self,
        jwt_token: &str,
        artifact: Artifact,
        base_path: PathBuf,
        run_id: &str,
    ) -> Result<()>;

    async fn get_devices_android(&self, jwt_token: &str) -> Result<Vec<AndroidDevice>>;
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
            base_url: String::from("https:://cloud.marathonlabs.io/api"),
            api_key: "".into(),
            client: Client::builder()
                .pool_idle_timeout(Some(Duration::from_secs(20)))
                .pool_max_idle_per_host(16)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl RapiClient for RapiReqwestClient {
    async fn get_token(&self) -> Result<String> {
        let url = format!("{}/v1/user/jwt", self.base_url);
        let params = [("api_key", self.api_key.clone())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;
        let response = self.client.get(url).send().await?;
        let response = api_error_adapter(response)
            .await?
            .json::<GetTokenResponse>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;
        Ok(response.token)
    }

    async fn create_run(
        &self,
        app: Option<PathBuf>,
        test_app: Option<PathBuf>,
        name: Option<String>,
        link: Option<String>,
        branch: Option<String>,
        platform: String,
        os_version: Option<String>,
        system_image: Option<String>,
        device: Option<String>,
        xcode_version: Option<String>,
        isolated: Option<bool>,
        code_coverage: Option<bool>,
        retry_quota_test_uncompleted: Option<u32>,
        retry_quota_test_preventive: Option<u32>,
        retry_quota_test_reactive: Option<u32>,
        analytics_read_only: Option<bool>,
        profiling: Option<bool>,
        filtering_configuration: Option<SparseMarathonfile>,
        no_progress_bar: bool,
        flavor: Option<String>,
        env_args: Option<Vec<String>>,
        test_env_args: Option<Vec<String>>,
        pull_file_config: Option<PullFileConfig>,
        concurrency_limit: Option<u32>,
        test_timeout_default: Option<u32>,
        test_timeout_max: Option<u32>,
        project: Option<String>,
        application_bundle: Option<Vec<ApplicationBundle>>,
        library_bundle: Option<Vec<PathBuf>>,
    ) -> Result<String> {
        let url = format!("{}/v2/run", self.base_url);
        let params = [("api_key", self.api_key.clone())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;

        let mut s3_test_app_path = None;
        if let Some(test_app) = test_app {
            s3_test_app_path = Some(
                upload_to_s3(
                    &self.client,
                    self.base_url.clone(),
                    self.api_key.clone(),
                    test_app.clone(),
                    no_progress_bar,
                )
                .await?,
            );
        }

        let mut s3_app_path = None;
        if let Some(app) = app {
            s3_app_path = Some(
                upload_to_s3(
                    &self.client,
                    self.base_url.clone(),
                    self.api_key.clone(),
                    app.clone(),
                    no_progress_bar,
                )
                .await?,
            );
        }

        let mut create_run_bundles: Vec<CreateRunBundle> = Vec::new();

        if let Some(app_bundles) = application_bundle {
            for app_bundle in app_bundles {
                let s3_app_path = upload_to_s3(
                    &self.client,
                    self.base_url.clone(),
                    self.api_key.clone(),
                    app_bundle.app_path.clone(),
                    no_progress_bar,
                )
                .await?;

                let s3_test_app_path = upload_to_s3(
                    &self.client,
                    self.base_url.clone(),
                    self.api_key.clone(),
                    app_bundle.test_app_path.clone(),
                    no_progress_bar,
                )
                .await?;

                let create_run_bundle = CreateRunBundle {
                    s3_app_path: Some(s3_app_path),
                    s3_test_app_path: s3_test_app_path.clone(),
                };
                create_run_bundles.push(create_run_bundle);
            }
        }

        if let Some(library_bundles) = library_bundle {
            for lib_bundle in library_bundles {
                let s3_test_app_path = upload_to_s3(
                    &self.client,
                    self.base_url.clone(),
                    self.api_key.clone(),
                    lib_bundle.clone(),
                    no_progress_bar,
                )
                .await?;

                let create_run_bundle = CreateRunBundle {
                    s3_app_path: None,
                    s3_test_app_path: s3_test_app_path.clone(),
                };
                create_run_bundles.push(create_run_bundle);
            }
        }

        let bundles = if create_run_bundles.is_empty() {
            None
        } else {
            Some(create_run_bundles)
        };

        let env_args_map = vec_to_hashmap(env_args)?;
        let test_env_args_map = vec_to_hashmap(test_env_args)?;

        let create_request = CreateRunRequest {
            s3_test_app_path: s3_test_app_path.clone(),
            platform: platform.clone(),
            s3_app_path: s3_app_path.clone(),
            analytics_read_only: analytics_read_only.clone(),
            profiling: profiling.clone(),
            code_coverage: code_coverage.clone(),
            concurrency_limit: concurrency_limit.clone(),
            country: None,
            device: device.clone(),
            filtering_configuration: filtering_configuration
                .map(|config| serde_json::to_string(&config).ok())
                .flatten(),
            flavor: flavor.clone(),
            isolated: isolated.clone(),
            language: None,
            link: link.clone(),
            name: name.clone(),
            branch: branch.clone(),
            os_version: os_version.clone(),
            project: project.clone(),
            pull_file_config: pull_file_config
                .map(|config| serde_json::to_string(&config).ok())
                .flatten(),
            retry_quota_test_preventive: retry_quota_test_preventive.clone(),
            retry_quota_test_reactive: retry_quota_test_reactive.clone(),
            retry_quota_test_uncompleted: retry_quota_test_uncompleted.clone(),
            system_image: system_image.clone(),
            xcode_version: xcode_version.clone(),
            test_timeout_default: test_timeout_default.clone(),
            test_timeout_max: test_timeout_max.clone(),
            env_args: env_args_map,
            test_env_args: test_env_args_map,
            bundles,
        };

        let response = self.client.post(url).json(&create_request).send().await?;
        let response = api_error_adapter(response)
            .await?
            .json::<CreateRunResponse>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;

        Ok(response.run_id)
    }

    async fn get_run(&self, id: &str) -> Result<TestRun> {
        let url = format!("{}/v1/run/{}", self.base_url, id);
        let params = [("api_key", self.api_key.clone())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;

        let response = self.client.get(url).send().await?;
        let response = api_error_adapter(response)
            .await?
            .json::<TestRun>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;
        Ok(response)
    }

    async fn list_artifact(&self, jwt_token: &str, id: &str) -> Result<Vec<Artifact>> {
        let url = format!("{}/v1/artifact/{}", self.base_url, id);

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await?;
        let response = api_error_adapter(response)
            .await?
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
        run_id: &str,
    ) -> Result<()> {
        let url = format!("{}/v1/artifact", self.base_url);
        let params = [("key", artifact.id.to_owned())];
        let url = reqwest::Url::parse_with_params(&url, &params)
            .map_err(|error| ApiError::InvalidParameters { error })?;

        let id = artifact.id.strip_prefix('/').unwrap_or(&artifact.id);
        let prefix_with_id = format!("{}/", run_id);
        let relative_path = artifact.id.strip_prefix(&prefix_with_id).unwrap_or(id);

        let relative_path = Path::new(&relative_path);
        let mut absolute_path = base_path.clone();
        absolute_path.push(relative_path);

        let src = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await?;

        let mut src = api_error_adapter(src).await?.bytes_stream();

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

    async fn get_devices_android(&self, jwt_token: &str) -> Result<Vec<AndroidDevice>> {
        let url = format!("{}/v1/devices/android", self.base_url);

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await?;
        let response = api_error_adapter(response)
            .await?
            .json::<Vec<AndroidDevice>>()
            .await
            .map_err(|error| ApiError::DeserializationFailure { error })?;

        Ok(response)
    }
}

fn vec_to_hashmap(
    vec: Option<Vec<String>>,
) -> Result<Option<HashMap<String, String>>, EnvArgError> {
    match vec {
        Some(args) => {
            let mut map = HashMap::new();
            for arg in args {
                let key_value: Vec<&str> = arg.splitn(2, '=').collect();
                if key_value.len() == 2 {
                    let key = key_value[0];
                    let value = key_value
                        .get(1)
                        .map(|val| val.to_string())
                        .unwrap_or_else(|| "".to_string());
                    if value.is_empty() {
                        return Err(EnvArgError::MissingValue {
                            env_arg: arg.clone(),
                        });
                    }
                    map.insert(key.to_string(), value.to_string());
                } else {
                    return Err(EnvArgError::InvalidKeyValue {
                        env_arg: arg.clone(),
                    });
                }
            }
            Ok(Some(map))
        }
        None => Ok(None),
    }
}

async fn api_error_adapter(response: reqwest::Response) -> Result<reqwest::Response> {
    match response.error_for_status_ref() {
        Ok(_) => Ok(response),
        Err(error) => {
            //Strip sensitive information
            let error = error.without_url();
            let body = response.text().await?;
            if let Some(status_code) = error.status() {
                match status_code {
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                        Err(ApiError::InvalidAuthenticationToken { error }.into())
                    }
                    _ => Err(ApiError::RequestFailedWithCode {
                        status_code,
                        error,
                        body,
                    }
                    .into()),
                }
            } else {
                Err(ApiError::RequestFailed { error }.into())
            }
        }
    }
}

async fn upload_to_s3(
    client: &Client,
    base_url_with_params: String,
    api_key: String,
    file_path: PathBuf,
    no_progress_bar: bool,
) -> Result<String> {
    // Open file
    let file = File::open(&file_path)
        .await
        .map_err(|error| InputError::OpenFileFailure {
            path: file_path.clone(),
            error,
        })?;

    // Extract filename from PathBuf
    let file_name = file_path
        .file_name()
        .map(|val| val.to_string_lossy().to_string())
        .ok_or(InputError::InvalidFileName {
            path: file_path.clone(),
        })?;

    // Request upload URL
    let url = format!("{}/v2/upload/presigned-url", base_url_with_params);
    let params = [("api_key", api_key.clone())];
    let url = reqwest::Url::parse_with_params(&url, &params)
        .map_err(|error| ApiError::InvalidParameters { error })?;

    let request_body = UploadRequest {
        filename: file_name.to_string(),
    };
    let upload_url_response = client.post(url).json(&request_body).send().await?;
    let upload_url_response = api_error_adapter(upload_url_response)
        .await?
        .json::<UploadUrlResponse>()
        .await
        .map_err(|error| ApiError::DeserializationFailure { error })?;

    // Progress stuff
    let file_total_size = file.metadata().await?.len();
    let mut file_reader = ReaderStream::new(file);
    let mut multi_progress: Option<MultiProgress> = if !no_progress_bar {
        Some(MultiProgress::new())
    } else {
        None
    };
    let file_progress_bar;
    let file_body;
    if !no_progress_bar {
        let sty = ProgressStyle::with_template(
            "{spinner:.blue} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        )
        .unwrap()
        .progress_chars("#>-");

        let pb = ProgressBar::new(file_total_size);
        pb.enable_steady_tick(Duration::from_millis(80));
        file_progress_bar = multi_progress.as_mut().unwrap().add(pb);
        file_progress_bar.set_style(sty.clone());
        let mut file_progress = 0u64;
        let file_stream = async_stream::stream! {
            while let Some(chunk) = file_reader.next().await {
                let file_progress_bar = file_progress_bar.clone();
                if let Ok(chunk) = &chunk {
                    let new = min(file_progress + (chunk.len() as u64), file_total_size);
                    file_progress = new;
                    file_progress_bar.set_position(new);
                    if file_progress >= file_total_size {
                        file_progress_bar.finish_and_clear();
                    }
                }
                yield chunk;
            }
        };
        file_body = Body::wrap_stream(file_stream);
    } else {
        file_body = Body::wrap_stream(file_reader);
    }

    let s3_response = client
        .put(upload_url_response.url.clone())
        .header("Content-Length", file_total_size)
        .body(file_body)
        .send()
        .await?;
    api_error_adapter(s3_response).await?;

    Ok(upload_url_response.file_path.clone())
}

#[derive(Serialize, Deserialize, Debug)]
struct UploadRequest {
    filename: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UploadUrlResponse {
    file_path: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[skip_serializing_none]
struct CreateRunRequest {
    #[serde(rename = "platform")]
    platform: String,

    #[serde(rename = "s3_test_app_path", default)]
    s3_test_app_path: Option<String>,
    #[serde(rename = "s3_app_path", default)]
    s3_app_path: Option<String>,
    #[serde(rename = "analytics_read_only", default)]
    analytics_read_only: Option<bool>,
    #[serde(rename = "profiling", default)]
    profiling: Option<bool>,
    #[serde(rename = "code_coverage", default)]
    code_coverage: Option<bool>,
    #[serde(rename = "concurrency_limit", default)]
    concurrency_limit: Option<u32>,
    #[serde(rename = "country", default)]
    country: Option<String>,
    #[serde(rename = "device", default)]
    device: Option<String>,
    #[serde(rename = "filtering_configuration", default)]
    filtering_configuration: Option<String>,
    #[serde(rename = "flavor", default)]
    flavor: Option<String>,
    #[serde(rename = "isolated", default)]
    isolated: Option<bool>,
    #[serde(rename = "language", default)]
    language: Option<String>,
    #[serde(rename = "link", default)]
    link: Option<String>,
    #[serde(rename = "name", default)]
    name: Option<String>,
    #[serde(rename = "branch", default)]
    branch: Option<String>,
    #[serde(rename = "os_version", default)]
    os_version: Option<String>,
    #[serde(rename = "project", default)]
    project: Option<String>,
    #[serde(rename = "pull_file_config", default)]
    pull_file_config: Option<String>,
    #[serde(rename = "retry_quota_test_preventive", default)]
    retry_quota_test_preventive: Option<u32>,
    #[serde(rename = "retry_quota_test_reactive", default)]
    retry_quota_test_reactive: Option<u32>,
    #[serde(rename = "retry_quota_test_uncompleted", default)]
    retry_quota_test_uncompleted: Option<u32>,
    #[serde(rename = "system_image", default)]
    system_image: Option<String>,
    #[serde(rename = "xcode_version", default)]
    xcode_version: Option<String>,
    #[serde(rename = "test_timeout_default", default)]
    test_timeout_default: Option<u32>,
    #[serde(rename = "test_timeout_max", default)]
    test_timeout_max: Option<u32>,
    #[serde(rename = "env_args", default)]
    env_args: Option<HashMap<String, String>>,
    #[serde(rename = "test_env_args", default)]
    test_env_args: Option<HashMap<String, String>>,
    #[serde(rename = "bundles", default)]
    bundles: Option<Vec<CreateRunBundle>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateRunBundle {
    #[serde(rename = "s3_test_app_path")]
    s3_test_app_path: String,

    #[serde(rename = "s3_app_path", skip_serializing_if = "Option::is_none")]
    s3_app_path: Option<String>,
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
    #[serde(rename = "total_run_time")]
    pub total_run_time_seconds: Option<f64>,
    #[serde(rename = "error_message")]
    pub error_message: Option<String>,
}

#[derive(Deserialize)]
pub struct GetTokenResponse {
    #[serde(rename = "token")]
    pub token: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Artifact {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "is_file")]
    pub is_file: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AndroidDevice {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "manufacturer")]
    pub manufacturer: String,
    #[serde(rename = "width")]
    pub width: u32,
    #[serde(rename = "height")]
    pub height: u32,
    #[serde(rename = "dpi")]
    pub dpi: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_vec_to_hashmap_valid_input() {
        let input = Some(vec![
            "KEY1=VALUE1".to_string(),
            "KEY2=VALUE2".to_string(),
            "KEY3=VALUE3".to_string(),
        ]);
        let mut expected = HashMap::new();
        expected.insert("KEY1".to_string(), "VALUE1".to_string());
        expected.insert("KEY2".to_string(), "VALUE2".to_string());
        expected.insert("KEY3".to_string(), "VALUE3".to_string());

        let result = vec_to_hashmap(input);

        assert_eq!(result, Ok(Some(expected)));
    }

    #[test]
    fn test_vec_to_hashmap_missing_value() {
        let input = Some(vec!["KEY1=VALUE1".to_string(), "KEY2=".to_string()]);

        let result = vec_to_hashmap(input);

        assert_eq!(
            result,
            Err(EnvArgError::MissingValue {
                env_arg: "KEY2=".to_string()
            })
        );
    }

    #[test]
    fn test_vec_to_hashmap_invalid_key_value() {
        let input = Some(vec!["KEY1=VALUE1".to_string(), "KEY2".to_string()]);

        let result = vec_to_hashmap(input);

        assert_eq!(
            result,
            Err(EnvArgError::InvalidKeyValue {
                env_arg: "KEY2".to_string()
            })
        );
    }

    #[test]
    fn test_vec_to_hashmap_none_input() {
        let input = None;

        let result = vec_to_hashmap(input);

        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_vec_to_hashmap_empty_vector() {
        let input = Some(vec![]);

        let result = vec_to_hashmap(input);

        assert_eq!(result, Ok(Some(HashMap::new())));
    }
}
