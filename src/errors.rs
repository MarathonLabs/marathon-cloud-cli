use std::{io::Write, path::PathBuf};

use console::Style;
use reqwest::{Error as ReqwestError, StatusCode};
use thiserror::Error;
use tokio::{io, task::JoinError};
use url::ParseError;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Invalid parameters for url")]
    InvalidParameters { error: ParseError },
    #[error("Failed to parse API response\nerror = {error}")]
    DeserializationFailure { error: reqwest::Error },
    #[error("API request failed\nerror = {error}")]
    RequestFailed { error: ReqwestError },
    #[error("API request failed\nstatus_code = {status_code}, error = {error}, body = {body}")]
    RequestFailedWithCode {
        status_code: StatusCode,
        error: ReqwestError,
        body: String,
    },
    #[error("Invalid authentication token. Did you supply correct API token?\nerror = {error}")]
    InvalidAuthenticationToken { error: ReqwestError },
}

#[derive(Error, Debug)]
pub enum EnvArgError {
    #[error("Invalid environment or testing environment variable. Double check you've supplied correct value\nvalue = {env_arg}")]
    InvalidKeyValue { env_arg: String },

    #[error("Invalid environment or testing environment variable. Value can not be empty \nvalue = {env_arg}")]
    MissingValue { env_arg: String },
}

#[derive(Error, Debug)]
pub enum ArtifactError {
    #[error("Failed to retrieve artifact list.\nerror = {error}")]
    ListFailed { error: JoinError },

    #[error("Failed to download artifacts.\nerror = {error}")]
    DownloadFailed { error: JoinError },
}

#[derive(Error, Debug)]
pub enum InputError {
    #[error("Invalid input file. Double check you've supplied correct path\npath = {path}")]
    InvalidFileName { path: PathBuf },

    #[error("Can't open file. Double check you've supplied correct path\npath = {path}")]
    OpenFileFailure { path: PathBuf, error: io::Error },

    #[error("Invalid xctestplan file: no test targets specified. Double check you've supplied correct path")]
    XctestplanMissingTargets,

    #[error("Invalid input file. All file paths should be valid UTF8\npath = {path}")]
    NonUTF8Path { path: PathBuf },

    #[error("Unsupported artifact format. Should be either {supported_files} file or {supported_folders} folder\npath = {path}")]
    UnsupportedArtifact {
        path: PathBuf,
        supported_files: String,
        supported_folders: String,
    },
}

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Unsupported run configuration: {message}")]
    UnsupportedRunConfiguration { message: String },
}

#[derive(Error, Debug)]
pub enum FilteringConfigurationError {
    #[error("Filter type {mtype} is not supported by Marathon Cloud")]
    UnsupportedFilterType { mtype: String },
    #[error("Filter type {mtype} is invalid")]
    InvalidFilterType { mtype: String },
    #[error("Invalid configuration for filter {mtype}: {message}")]
    InvalidFilterConfiguration { mtype: String, message: String },
}

pub fn default_error_handler(
    error: Box<dyn std::error::Error + Send + 'static>,
    output: &mut dyn Write,
) {
    let red = Style::new().red();
    _ = writeln!(output, "{}", red.apply_to(error));
}
