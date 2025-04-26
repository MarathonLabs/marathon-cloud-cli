use crate::{errors::InputError, pull::parse_pull_args};
use anyhow::Result;
use std::{fmt::Display, path::PathBuf};

use crate::{
    bundle,
    cli::{self, AnalyticsArgs, ApiArgs, CommonRunArgs, RetryArgs},
    errors::ConfigurationError,
    filtering,
    interactor::TriggerTestRunInteractor,
    pull::PullFileConfig,
};

use super::ProfilingArgs;

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum SystemImage {
    #[clap(name = "default")]
    Default,
    #[clap(name = "google_apis")]
    GoogleApis,
    #[clap(name = "google_apis_playstore")]
    GoogleApisPlaystore,
}

impl Display for SystemImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemImage::Default => f.write_str("default"),
            SystemImage::GoogleApis => f.write_str("google_apis"),
            SystemImage::GoogleApisPlaystore => f.write_str("google_apis_playstore"),
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
    #[clap(name = "15")]
    Android15,
}

impl Display for OsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsVersion::Android10 => f.write_str("10"),
            OsVersion::Android11 => f.write_str("11"),
            OsVersion::Android12 => f.write_str("12"),
            OsVersion::Android13 => f.write_str("13"),
            OsVersion::Android14 => f.write_str("14"),
            OsVersion::Android15 => f.write_str("15"),
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
    test_application: Option<std::path::PathBuf>,
    os_version: Option<OsVersion>,
    system_image: Option<SystemImage>,
    device: Option<String>,
    common: CommonRunArgs,
    api_args: ApiArgs,
    flavor: Option<Flavor>,
    instrumentation_arg: Option<Vec<String>>,
    retry_args: RetryArgs,
    analytics_args: AnalyticsArgs,
    profiling_args: ProfilingArgs,
    pull_files: Option<Vec<String>>,
    application_bundle: Option<Vec<String>>,
    library_bundle: Option<Vec<PathBuf>>,
    mock_location: bool,
) -> Result<bool> {
    if application.is_none()
        && test_application.is_none()
        && application_bundle.is_none()
        && library_bundle.is_none()
    {
        return Err(ConfigurationError::UnsupportedRunConfiguration {
            message:
                "Please set up APKs for testing. The following argument combinations are possible:
--application <APPLICATION> --test-application <TEST_APPLICATION> - for application testing
--application-bundle <APPLICATION>,<TEST_APPLICATION> - advanced mode that allows setting up one or more application bundles for testing
--library-bundle <TEST_APPLICATION> - advanced mode that allows setting up one or more library bundles for testing"
                    .into(),
        }
        .into());
    }

    if application.is_some()
        && test_application.is_none()
        && application_bundle.is_none()
        && library_bundle.is_none()
    {
        return Err(ConfigurationError::UnsupportedRunConfiguration {
            message: "Please set up Testing APK:
--test-application <TEST_APPLICATION>"
                .into(),
        }
        .into());
    }

    if application.is_none()
        && test_application.is_some()
        && application_bundle.is_none()
        && library_bundle.is_none()
    {
        return Err(ConfigurationError::UnsupportedRunConfiguration {
            message: "Please set up Application APK:
--application <TEST_APPLICATION>
If you are interesting in library testing then please use advance mode with --library-bundle argument"
                .into(),
        }
        .into());
    }

    if let Some(bundles) = &application_bundle {
        if bundles.len() > 1 && mock_location {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                message: "Mock location access doesn't support multiple application bundles".into(),
            }
            .into());
        }
    }

    if application_bundle.is_none() && application.is_none() && mock_location {
        return Err(ConfigurationError::UnsupportedRunConfiguration {
            message: "There is no Application where mock location can be used".into(),
        }
        .into());
    }

    match (device.as_deref(), &flavor, &system_image, &os_version) {
        (Some("watch"), _, Some(SystemImage::Default) | None, Some(_) | None)
        | (
            Some("watch"),
            _,
            Some(_),
            Some(OsVersion::Android10)
            | Some(OsVersion::Android12)
            | Some(OsVersion::Android14)
            | Some(OsVersion::Android15),
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
        (Some("tv"), _, _, Some(OsVersion::Android15)) => {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                message: "Android TV doesn't support os version 15".into(),
            }
            .into());
        }
        (Some("wear"), _, _, Some(OsVersion::Android15)) => {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                message: "Android Wear doesn't support os version 15".into(),
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
        (_, _, Some(SystemImage::Default) | None, Some(OsVersion::Android15)) => {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                message: "Android OS version 15 only supports google_apis or google_apis_playstore system image".into(),
            }
            .into());
        }
        _ => {}
    }

    if let Some(app_path) = application.clone() {
        if !app_path.exists() {
            return Err(InputError::InvalidFileName { path: app_path })?;
        }
    }

    if let Some(app_path) = test_application.clone() {
        if !app_path.exists() {
            return Err(InputError::InvalidFileName { path: app_path })?;
        }
    }

    let mut transformed_application_bundle = None;
    if let Some(application_bundle) = application_bundle {
        transformed_application_bundle =
            Some(bundle::transform_and_validate_bundle(application_bundle)?);
    }

    if let Some(lib_bundles) = library_bundle.clone() {
        for bundle in lib_bundles {
            if !bundle.exists() {
                return Err(InputError::InvalidFileName { path: bundle })?;
            }
        }
    }

    let filter_file = common.filter_file.map(filtering::convert::convert);
    let filtering_configuration = match filter_file {
        Some(future) => Some(future.await?),
        None => None,
    };

    let retry_args = cli::validate::retry_args(retry_args);
    cli::validate::result_file_args(&common.result_file_args)?;

    let pull_file_config: Option<PullFileConfig> = match pull_files {
        Some(args) => Some(parse_pull_args(args)?),
        None => None,
    };

    if let Some(limit) = common.concurrency_limit {
        if limit == 0 {
            return Err(InputError::NonPositiveValue {
                arg: "--concurrency-limit".to_owned(),
            })?;
        }
    }

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
            common.branch,
            present_wait,
            common.isolated,
            common.ignore_test_failures,
            common.code_coverage,
            retry_args.retry_quota_test_uncompleted,
            retry_args.retry_quota_test_preventive,
            retry_args.retry_quota_test_reactive,
            analytics_args.analytics_read_only,
            profiling_args.profiling,
            mock_location,
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
            pull_file_config,
            common.concurrency_limit,
            None,
            None,
            common.project,
            transformed_application_bundle,
            library_bundle,
            None,
        )
        .await
}
