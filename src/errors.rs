use std::path::PathBuf;

use reqwest::Error as ReqwestError;
use thiserror::Error;
use tokio::{task::JoinError, io};
use url::ParseError;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Unauthorized client. Double check you've supplied correct api key or you have appropriate permissions\nerror = ${error}")]
    Unauthorized { error: ReqwestError },
    #[error("Invalid parameters for url")]
    InvalidParameters { error: ParseError },
    #[error("Failed to parse API response\nerror = ${error}")]
    DeserializationFailure { error: reqwest::Error },
    #[error("API request failed\nerror = ${error}")]
    RequestFailed { error: ReqwestError },
}

#[derive(Error, Debug)]
pub enum ArtifactError {
    #[error("Failed to retrieve artifact list.\nerror = ${error}")]
    ListFailed { error: JoinError },

    #[error("Failed to download artifacts.\nerror = ${error}")]
    DownloadFailed { error: JoinError },
}

#[derive(Error, Debug)]
pub enum InputError {
    #[error("Invalid input file. Double check you've supplied correct path\npath= ${path}")]
    InvalidFileName { path: PathBuf },

    #[error("Can't open file. Double check you've supplied correct path\npath= ${path}")]
    OpenFileFailure { path: PathBuf, error: io::Error },

}
