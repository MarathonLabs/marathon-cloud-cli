use std::ffi::OsStr;
use std::fmt::Display;

use anyhow::Result;
use tokio::fs::File;
use walkdir::WalkDir;

use crate::{
    cli::{self},
    compression,
    errors::ConfigurationError,
    interactor::TriggerTestRunInteractor,
};
use crate::{errors::InputError, filtering};

#[derive(Debug, clap::ValueEnum, Clone, PartialEq, Eq)]
pub enum IosDevice {
    #[clap(name = "iPhone-14")]
    IPhone14,
    #[clap(name = "iPhone-15")]
    IPhone15,
    #[clap(name = "iPhone-15-Pro")]
    IPhone15Pro,
    #[clap(name = "iPhone-15-Pro-Max")]
    IPhone15ProMax,
}

impl Display for IosDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IosDevice::IPhone14 => f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-14"),
            IosDevice::IPhone15 => f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-15"),
            IosDevice::IPhone15Pro => {
                f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-15-Pro")
            }
            IosDevice::IPhone15ProMax => {
                f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-15-Pro-Max")
            }
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone, PartialEq, Eq)]
pub enum OsVersion {
    #[clap(name = "16.4")]
    Ios16_4,
    #[clap(name = "17.2")]
    Ios17_2,
    #[clap(name = "17.5")]
    Ios17_5,
}

impl Display for OsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsVersion::Ios16_4 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-16-4"),
            OsVersion::Ios17_2 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-17-2"),
            OsVersion::Ios17_5 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-17-5"),
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone, PartialEq, Eq)]
pub enum XcodeVersion {
    #[clap(name = "14.3.1")]
    Xcode14_3_1,
    #[clap(name = "15.2")]
    Xcode15_2,
    #[clap(name = "15.4")]
    Xcode15_4,
}

impl Display for XcodeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XcodeVersion::Xcode14_3_1 => f.write_str("14.3.1"),
            XcodeVersion::Xcode15_2 => f.write_str("15.2"),
            XcodeVersion::Xcode15_4 => f.write_str("15.4"),
        }
    }
}

pub(crate) async fn ensure_format(path: std::path::PathBuf) -> Result<std::path::PathBuf> {
    let supported_extensions_file = vec!["zip", "ipa"];
    let supported_extensions_dir = vec!["app", "xctest"];
    if path.is_file()
        && path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|ext| supported_extensions_file.contains(&ext))
    {
        Ok(path)
    } else if path.is_dir()
        && path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|ext| supported_extensions_dir.contains(&ext))
    {
        let dst = &path.with_extension("zip");
        let dst_file = File::create(dst).await?;

        let walkdir = WalkDir::new(&path);
        let it = walkdir.into_iter();
        let prefix = &path
            .parent()
            .unwrap_or(&path)
            .to_str()
            .ok_or(InputError::NonUTF8Path { path: path.clone() })?;

        compression::zip_dir(&mut it.filter_map(|e| e.ok()), prefix, dst_file).await?;
        Ok(dst.to_owned())
    } else {
        Err(InputError::UnsupportedArtifact {
            path,
            supported_files: "[ipa,zip]".into(),
            supported_folders: "[app,xctest]".into(),
        }
        .into())
    }
}

pub(crate) fn get_supported_configs(
) -> Vec<(Option<IosDevice>, Option<XcodeVersion>, Option<OsVersion>)> {
    vec![
        (
            Some(IosDevice::IPhone14),
            Some(XcodeVersion::Xcode14_3_1),
            Some(OsVersion::Ios16_4),
        ),
        (
            Some(IosDevice::IPhone15),
            Some(XcodeVersion::Xcode15_2),
            Some(OsVersion::Ios17_2),
        ),
        (
            Some(IosDevice::IPhone15Pro),
            Some(XcodeVersion::Xcode15_2),
            Some(OsVersion::Ios17_2),
        ),
        (
            Some(IosDevice::IPhone15ProMax),
            Some(XcodeVersion::Xcode15_2),
            Some(OsVersion::Ios17_2),
        ),
        (
            Some(IosDevice::IPhone15),
            Some(XcodeVersion::Xcode15_4),
            Some(OsVersion::Ios17_5),
        ),
    ]
}

pub(crate) async fn infer_parameters(
    device: Option<IosDevice>,
    xcode_version: Option<XcodeVersion>,
    os_version: Option<OsVersion>,
) -> (Option<IosDevice>, Option<XcodeVersion>, Option<OsVersion>) {
    let supported_configs = get_supported_configs();
    let (mut device, mut xcode_version, mut os_version) = (device, xcode_version, os_version);
    for (d, x, o) in &supported_configs {
        if let Some(dev) = &device {
            if d == &Some(dev.clone()) {
                xcode_version = xcode_version.or_else(|| x.clone());
                os_version = os_version.or_else(|| o.clone());
                break;
            }
        }
        if let Some(xcode) = &xcode_version {
            if x == &Some(xcode.clone()) {
                device = device.or_else(|| d.clone());
                os_version = os_version.or_else(|| o.clone());
                break;
            }
        }
        if let Some(os) = &os_version {
            if o == &Some(os.clone()) {
                device = device.or_else(|| d.clone());
                xcode_version = xcode_version.or_else(|| x.clone());
                break;
            }
        }
    }

    (device, xcode_version, os_version)
}

pub(crate) async fn run(
    application: std::path::PathBuf,
    test_application: std::path::PathBuf,
    os_version: Option<OsVersion>,
    device: Option<IosDevice>,
    xcode_version: Option<XcodeVersion>,
    common: super::CommonRunArgs,
    api_args: super::ApiArgs,
    xctestrun_env: Option<Vec<String>>,
    xctestrun_test_env: Option<Vec<String>>,
    xctestplan_filter_file: Option<std::path::PathBuf>,
    xctestplan_target_name: Option<String>,
    retry_args: super::RetryArgs,
    analytics_args: super::AnalyticsArgs,
    test_timeout_default: Option<u32>,
    test_timeout_max: Option<u32>,
) -> Result<bool> {
    let supported_configs = get_supported_configs();
    let (device, xcode_version, os_version) =
        infer_parameters(device, xcode_version, os_version).await;

    // Existing match statement with inferred values
    match (&device, &xcode_version, &os_version) {
        (None, None, None) => {}
        _ if supported_configs.contains(&(
            device.clone(),
            xcode_version.clone(),
            os_version.clone(),
        )) => {}
        _ => {
            return Err(ConfigurationError::UnsupportedRunConfiguration {
                                    message: "
Please set --xcode-version, --os-version, and --device correctly.
Supported iOS settings combinations are:
    --xcode_version 14.3.1 --os-version 16.4 --device iPhone-14 => Default
    --xcode_version 15.2 --os-version 17.2 --device [iPhone-15, iPhone-15-Pro, iPhone-15-Pro-Max]
    --xcode_version 15.4 --os-version 17.5 --device [iPhone-15]
If you provide any single or two of these parameters, the others will be inferred based on supported combinations."
                                        .into(),
                                }
                                .into());
        }
    }

    let filtering_configuration = if xctestplan_filter_file.is_some() {
        Some(
            filtering::convert::convert_xctestplan(
                xctestplan_filter_file.unwrap(),
                xctestplan_target_name,
            )
            .await?,
        )
    } else {
        let filter_file = common.filter_file.map(filtering::convert::convert);
        match filter_file {
            Some(future) => Some(future.await?),
            None => None,
        }
    };
    let application = ensure_format(application).await?;
    let test_application = ensure_format(test_application).await?;

    let retry_args = cli::validate::retry_args(retry_args);
    cli::validate::result_file_args(&common.result_file_args)?;

    if let Some(limit) = common.concurrency_limit {
        if limit == 0 {
            return Err(InputError::NonPositiveValue {
                arg: "--concurrency-limit".to_owned(),
            })?;
        }
    }

    if let Some(limit) = test_timeout_default {
        if limit == 0 {
            return Err(InputError::NonPositiveValue {
                arg: "--test-timeout-default".to_owned(),
            })?;
        }
    }

    if let Some(limit) = test_timeout_max {
        if limit == 0 {
            return Err(InputError::NonPositiveValue {
                arg: "--test-timeout-max".to_owned(),
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
            filtering_configuration,
            &common.output,
            Some(application),
            Some(test_application),
            os_version.map(|x| x.to_string()),
            None,
            device.map(|x| x.to_string()),
            xcode_version.map(|x| x.to_string()),
            None,
            "iOS".to_owned(),
            common.progress_args.no_progress_bars,
            common.result_file_args.result_file,
            xctestrun_env,
            xctestrun_test_env,
            None,
            common.concurrency_limit,
            test_timeout_default,
            test_timeout_max,
            common.project,
            None,
            None,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_infer_parameters_device_provided() {
        let provided_device = Some(IosDevice::IPhone14);
        let expected_xcode_version = Some(XcodeVersion::Xcode14_3_1);
        let expected_os_version = Some(OsVersion::Ios16_4);

        let (inferred_device, inferred_xcode_version, inferred_os_version) =
            infer_parameters(provided_device, None, None).await;

        assert_eq!(inferred_device, Some(IosDevice::IPhone14));
        assert_eq!(inferred_xcode_version, expected_xcode_version);
        assert_eq!(inferred_os_version, expected_os_version);
    }

    #[tokio::test]
    async fn test_infer_parameters_device_iphone15_provided() {
        let provided_device = Some(IosDevice::IPhone15);
        let expected_xcode_version = Some(XcodeVersion::Xcode15_2);
        let expected_os_version = Some(OsVersion::Ios17_2);

        let (inferred_device, inferred_xcode_version, inferred_os_version) =
            infer_parameters(provided_device, None, None).await;

        assert_eq!(inferred_device, Some(IosDevice::IPhone15));
        assert_eq!(inferred_xcode_version, expected_xcode_version);
        assert_eq!(inferred_os_version, expected_os_version);
    }

    #[tokio::test]
    async fn test_infer_parameters_xcode_version_provided() {
        let provided_xcode_version = Some(XcodeVersion::Xcode15_2);
        let expected_device = Some(IosDevice::IPhone15);
        let expected_os_version = Some(OsVersion::Ios17_2);

        let (inferred_device, inferred_xcode_version, inferred_os_version) =
            infer_parameters(None, provided_xcode_version, None).await;

        assert_eq!(inferred_device, expected_device);
        assert_eq!(inferred_xcode_version, Some(XcodeVersion::Xcode15_2));
        assert_eq!(inferred_os_version, expected_os_version);
    }

    #[tokio::test]
    async fn test_infer_parameters_os_version_provided() {
        let provided_os_version = Some(OsVersion::Ios16_4);
        let expected_device = Some(IosDevice::IPhone14);
        let expected_xcode_version = Some(XcodeVersion::Xcode14_3_1);

        let (inferred_device, inferred_xcode_version, inferred_os_version) =
            infer_parameters(None, None, provided_os_version).await;

        assert_eq!(inferred_device, expected_device);
        assert_eq!(inferred_xcode_version, expected_xcode_version);
        assert_eq!(inferred_os_version, Some(OsVersion::Ios16_4));
    }

    #[tokio::test]
    async fn test_infer_parameters_device_and_xcode_version_provided() {
        let provided_device = Some(IosDevice::IPhone14);
        let provided_xcode_version = Some(XcodeVersion::Xcode14_3_1);
        let expected_os_version = Some(OsVersion::Ios16_4);

        let (inferred_device, inferred_xcode_version, inferred_os_version) = infer_parameters(
            provided_device.clone(),
            provided_xcode_version.clone(),
            None,
        )
        .await;

        // Check if the provided parameters are unchanged
        assert_eq!(inferred_device, provided_device);
        assert_eq!(inferred_xcode_version, provided_xcode_version);

        // Check if the missing parameter is correctly inferred
        assert_eq!(inferred_os_version, expected_os_version);
    }
}
