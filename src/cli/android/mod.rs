use anyhow::Result;
use std::fmt::Display;

use crate::{
    cli::{self, AnalyticsArgs, ApiArgs, CommonRunArgs, RetryArgs},
    errors::ConfigurationError,
    filtering,
    interactor::TriggerTestRunInteractor,
};

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum SystemImage {
    #[clap(name = "default")]
    Default,
    #[clap(name = "google_apis")]
    GoogleApis,
}

impl Display for SystemImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemImage::Default => f.write_str("default"),
            SystemImage::GoogleApis => f.write_str("google_apis"),
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum OsVersion {
    #[clap(name = "10")]
    Android10,
    #[clap(name = "11")]
    Android11,
    #[clap(name = "12")]
    Android12,
    #[clap(name = "13")]
    Android13,
    #[clap(name = "14")]
    Android14,
}

impl Display for OsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsVersion::Android10 => f.write_str("10"),
            OsVersion::Android11 => f.write_str("11"),
            OsVersion::Android12 => f.write_str("12"),
            OsVersion::Android13 => f.write_str("13"),
            OsVersion::Android14 => f.write_str("14"),
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum Flavor {
    #[clap(name = "native")]
    Native,
    #[clap(name = "js-jest-appium")]
    JsJestAppium,
    #[clap(name = "python-robotframework-appium")]
    PythonRobotFrameworkAppium,
}

impl Display for Flavor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Flavor::Native => f.write_str("native"),
            Flavor::JsJestAppium => f.write_str("js-jest-appium"),
            Flavor::PythonRobotFrameworkAppium => f.write_str("python-robotframework-appium"),
        }
    }
}

pub(crate) async fn run(
    application: Option<std::path::PathBuf>,
    test_application: std::path::PathBuf,
    os_version: Option<OsVersion>,
    system_image: Option<SystemImage>,
    device: Option<String>,
    common: CommonRunArgs,
    api_args: ApiArgs,
    flavor: Option<Flavor>,
    instrumentation_arg: Option<Vec<String>>,
    retry_args: RetryArgs,
    analytics_args: AnalyticsArgs,
) -> Result<bool> {
    match (device.as_deref(), &flavor, &system_image, &os_version) {
        (Some("watch"), _, Some(SystemImage::Default) | None, Some(_) | None)
        | (
            Some("watch"),
            _,
            Some(_),
            Some(OsVersion::Android10) | Some(OsVersion::Android12) | Some(OsVersion::Android14),
        ) => {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                message:
                    "Android Watch only supports google-apis system image and os versions 11 and 13"
                        .into(),
            }
            .into());
        }
        (Some("tv"), _, Some(SystemImage::Default), Some(_) | None) => {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                message: "Android TV only supports google-apis system image".into(),
            }
            .into());
        }
        (
            Some("tv") | Some("watch"),
            Some(Flavor::JsJestAppium) | Some(Flavor::PythonRobotFrameworkAppium),
            _,
            _,
        ) => {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                message:
                    "js-jest-appium and python-robotframework-appium only support 'phone' devices"
                        .into(),
            }
            .into());
        }
        _ => {}
    }

    let filter_file = common.filter_file.map(filtering::convert::convert);
    let filtering_configuration = match filter_file {
        Some(future) => Some(future.await?),
        None => None,
    };

    let retry_args = cli::validate::retry_args(retry_args);
    cli::validate::result_file_args(&common.result_file_args)?;

    let present_wait: bool = match common.wait {
        None => true,
        Some(true) => true,
        Some(false) => false,
    };

    TriggerTestRunInteractor {}
        .execute(
            &api_args.base_url,
            &api_args.api_key,
            common.name,
            common.link,
            present_wait,
            common.isolated,
            common.ignore_test_failures,
            common.code_coverage,
            retry_args.retry_quota_test_uncompleted,
            retry_args.retry_quota_test_preventive,
            retry_args.retry_quota_test_reactive,
            analytics_args.analytics_read_only,
            filtering_configuration,
            &common.output,
            application,
            test_application,
            os_version.map(|x| x.to_string()),
            system_image.map(|x| x.to_string()),
            device,
            None,
            flavor.map(|x| x.to_string()),
            "Android".to_owned(),
            common.progress_args.no_progress_bars,
            common.result_file_args.result_file,
            instrumentation_arg,
            None,
        )
        .await
}
