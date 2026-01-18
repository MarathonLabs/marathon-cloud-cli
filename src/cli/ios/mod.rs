pub mod maestro;

use std::fmt::Display;
use std::time::Duration;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use log::warn;
use std::collections::HashSet;

use crate::cli::validate;
use crate::formatter::Formatter;
use crate::{
    cli::{self},
    errors::ConfigurationError,
    formatter::StandardFormatter,
    hash,
    interactor::TriggerTestRunInteractor,
};
use crate::{errors::InputError, filtering};

#[derive(Debug, clap::ValueEnum, Clone, PartialEq, Eq)]
pub enum IosDevice {
    #[clap(name = "iPhone-11")]
    IPhone11,
    #[clap(name = "iPhone-15")]
    IPhone15,
    #[clap(name = "iPhone-15-Pro")]
    IPhone15Pro,
    #[clap(name = "iPhone-15-Pro-Max")]
    IPhone15ProMax,
    #[clap(name = "iPhone-16")]
    IPhone16,
    #[clap(name = "iPhone-16-Pro")]
    IPhone16Pro,
    #[clap(name = "iPhone-16-Pro-Max")]
    IPhone16ProMax,
    #[clap(name = "iPhone-16-Plus")]
    IPhone16Plus,
}

impl Display for IosDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IosDevice::IPhone11 => f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-11"),
            IosDevice::IPhone15 => f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-15"),
            IosDevice::IPhone15Pro => {
                f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-15-Pro")
            }
            IosDevice::IPhone15ProMax => {
                f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-15-Pro-Max")
            }
            IosDevice::IPhone16 => f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-16"),
            IosDevice::IPhone16Pro => {
                f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-16-Pro")
            }
            IosDevice::IPhone16ProMax => {
                f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-16-Pro-Max")
            }
            IosDevice::IPhone16Plus => {
                f.write_str("com.apple.CoreSimulator.SimDeviceType.iPhone-16-Plus")
            }
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone, PartialEq, Eq)]
pub enum OsVersion {
    #[clap(name = "17.5")]
    Ios17_5,
    #[clap(name = "18.2")]
    Ios18_2,
    #[clap(name = "18.4")]
    Ios18_4,
    #[clap(name = "26.1")]
    Ios26_1,
}

impl Display for OsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsVersion::Ios17_5 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-17-5"),
            OsVersion::Ios18_2 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-18-2"),
            OsVersion::Ios18_4 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-18-4"),
            OsVersion::Ios26_1 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-26-1"),
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone, PartialEq, Eq)]
pub enum XcodeVersion {
    #[clap(name = "15.4")]
    Xcode15_4,
    #[clap(name = "16.2")]
    Xcode16_2,
    #[clap(name = "16.3")]
    Xcode16_3,
    #[clap(name = "26.1")]
    Xcode26_1,
}

impl Display for XcodeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XcodeVersion::Xcode15_4 => f.write_str("15.4"),
            XcodeVersion::Xcode16_2 => f.write_str("16.2"),
            XcodeVersion::Xcode16_3 => f.write_str("16.3"),
            XcodeVersion::Xcode26_1 => f.write_str("26.1"),
        }
    }
}

pub(crate) fn get_supported_configs() -> Vec<(Option<IosDevice>, Option<OsVersion>)> {
    vec![
        (Some(IosDevice::IPhone15), Some(OsVersion::Ios17_5)),
        (Some(IosDevice::IPhone15Pro), Some(OsVersion::Ios17_5)),
        (Some(IosDevice::IPhone15ProMax), Some(OsVersion::Ios17_5)),
        (Some(IosDevice::IPhone11), Some(OsVersion::Ios17_5)),
        (Some(IosDevice::IPhone16), Some(OsVersion::Ios18_2)),
        (Some(IosDevice::IPhone16Pro), Some(OsVersion::Ios18_2)),
        (Some(IosDevice::IPhone16ProMax), Some(OsVersion::Ios18_2)),
        (Some(IosDevice::IPhone16Plus), Some(OsVersion::Ios18_2)),
        (Some(IosDevice::IPhone11), Some(OsVersion::Ios18_2)),
        (Some(IosDevice::IPhone16), Some(OsVersion::Ios18_4)),
        (Some(IosDevice::IPhone16Pro), Some(OsVersion::Ios18_4)),
        (Some(IosDevice::IPhone16ProMax), Some(OsVersion::Ios18_4)),
        (Some(IosDevice::IPhone16Plus), Some(OsVersion::Ios18_4)),
        (Some(IosDevice::IPhone11), Some(OsVersion::Ios18_4)),
        (Some(IosDevice::IPhone16Pro), Some(OsVersion::Ios26_1)),
        (Some(IosDevice::IPhone16Pro), Some(OsVersion::Ios18_4)),
        (Some(IosDevice::IPhone16Pro), Some(OsVersion::Ios18_2)),
        (Some(IosDevice::IPhone16Pro), Some(OsVersion::Ios17_5)),
    ]
}

pub(crate) async fn infer_parameters(
    device: Option<IosDevice>,
    os_version: Option<OsVersion>,
) -> Result<(IosDevice, OsVersion)> {
    let supported_configs = get_supported_configs();

    // Filter out configurations that match the provided parameters
    let filtered_configs: Vec<&(Option<IosDevice>, Option<OsVersion>)> = supported_configs
        .iter()
        .filter(|(d, o)| {
            (device.is_none() || d == &device) && (os_version.is_none() || o == &os_version)
        })
        .collect();

    // If no valid configuration is found, return an error
    if filtered_configs.is_empty() {
        return Err(anyhow::anyhow!("Invalid parameters"));
    }

    // If only one valid configuration is found, use it
    if filtered_configs.len() == 1 {
        let (final_device, final_os) = filtered_configs[0];
        return Ok((final_device.clone().unwrap(), final_os.clone().unwrap()));
    }

    // If multiple configurations are still valid, we need more specific parameters
    if filtered_configs.len() > 1 {
        return Err(anyhow::anyhow!(
            "Ambiguous parameters, please provide more specific input."
        ));
    }

    Ok((device.unwrap(), os_version.unwrap()))
}

fn get_allowed_permissions() -> HashSet<&'static str> {
    HashSet::from([
        "calendar",
        "contacts-limited",
        "contacts",
        "location",
        "location-always",
        "photos-add",
        "photos",
        "media-library",
        "microphone",
        "motion",
        "reminders",
        "siri",
    ])
}

#[allow(clippy::too_many_arguments)]
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
    granted_permission: Option<Vec<String>>,
) -> Result<bool> {
    let (device, os_version) = match validate_device_configuration(os_version, device).await {
        Ok(value) => value,
        Err(value) => return value,
    };

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

    let application =
        validate::ensure_format(&application, &["zip", "ipa"], &["app"], true).await?;
    let test_application =
        validate::ensure_format(&test_application, &["zip", "ipa"], &["app", "xctest"], true)
            .await?;

    let present_wait: bool = match common.wait {
        None => true,
        Some(true) => true,
        Some(false) => false,
    };

    let steps = match (&present_wait, &common.output) {
        (true, Some(_)) => 6,
        (true, None) => 3,
        _ => 2,
    };
    let mut formatter = StandardFormatter::new(steps);
    formatter.stage("Validating input...");
    let spinner = if !common.progress_args.no_progress_bars {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]),
        );
        pb.set_message("Validating input...");
        Some(pb)
    } else {
        None
    };
    if xcode_version.is_some() {
        warn!("Specifying xcode version has been deprecated")
    }

    let application = hash::md5(application).await?;
    let test_application = hash::md5(test_application).await?;

    if let Some(s) = spinner {
        s.finish_and_clear()
    }

    if application.md5 == test_application.md5 {
        return Err(InputError::DuplicatedApplicationBundle {
            app: application.path.clone(),
            test: test_application.path.clone(),
        })?;
    }

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

    if let Some(granted_permission) = granted_permission.clone() {
        let allowed_permissions = get_allowed_permissions();
        let invalid_permissions: Vec<_> = granted_permission
            .iter()
            .filter(|perm| !allowed_permissions.contains(perm.as_str()))
            .cloned()
            .collect();

        if !invalid_permissions.is_empty() {
            return Err(InputError::IncorrectPermission {
                permissions: invalid_permissions,
            })?;
        }
    }

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
            false,
            false,
            filtering_configuration,
            &common.output,
            Some(application),
            Some(test_application),
            None,
            os_version.map(|x| x.to_string()),
            None,
            device.map(|x| x.to_string()),
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
            granted_permission,
            formatter,
        )
        .await
}

async fn validate_device_configuration(
    os_version: Option<OsVersion>,
    device: Option<IosDevice>,
) -> Result<(Option<IosDevice>, Option<OsVersion>), std::result::Result<bool, anyhow::Error>> {
    let (device, os_version) = if device.is_none() && os_version.is_none() {
        (None, None)
    } else {
        match infer_parameters(device, os_version).await {
            Ok((dev, os)) => (Some(dev), Some(os)),
            Err(_) => {
                return Err(Err(ConfigurationError::UnsupportedRunConfiguration {
                    message: "
Please set --xcode-version, --os-version, and --device correctly.
Supported iOS settings combinations are:
    --os-version 17.5 --device iPhone-15 => Default
    --os-version 17.5 --device iPhone-15-Pro
    --os-version 17.5 --device iPhone-15-Pro-Max
    --os-version 17.5 --device iPhone-11
    --os-version 18.2 --device iPhone-16
    --os-version 18.2 --device iPhone-16-Pro
    --os-version 18.2 --device iPhone-16-Pro-Max
    --os-version 18.2 --device iPhone-16-Plus
    --os-version 18.2 --device iPhone-11
    --os-version 18.4 --device iPhone-16
    --os-version 18.4 --device iPhone-16-Pro
    --os-version 18.4 --device iPhone-16-Pro-Max
    --os-version 18.4 --device iPhone-16-Plus
    --os-version 18.4 --device iPhone-11
First example: If you choose --xcode-version 15.4 --device iPhone-15-Pro then the --os-version will be inferred (17.5).
Second example: If you choose --device iPhone-11 then you will receive an error because --os-version param is ambiguous."
                        .into(),
                }.into()));
            }
        }
    };
    Ok((device, os_version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_infer_parameters_device_and_xcode_version_provided() -> Result<()> {
        let provided_device = Some(IosDevice::IPhone15);
        let expected_os_version = OsVersion::Ios17_5;

        let (inferred_device, inferred_os_version) =
            infer_parameters(provided_device, None).await?;

        assert_eq!(inferred_device, IosDevice::IPhone15);
        assert_eq!(inferred_os_version, expected_os_version);

        Ok(())
    }

    #[tokio::test]
    async fn test_infer_parameters_ambiguous_xcode_version_should_error() {
        let result = infer_parameters(None, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_infer_parameters_complete_input_valid() -> Result<()> {
        let provided_device = Some(IosDevice::IPhone15);
        let provided_os_version = Some(OsVersion::Ios17_5);

        let (inferred_device, inferred_os_version) =
            infer_parameters(provided_device, provided_os_version).await?;

        assert_eq!(inferred_device, IosDevice::IPhone15);
        assert_eq!(inferred_os_version, OsVersion::Ios17_5);

        Ok(())
    }

    #[tokio::test]
    async fn test_infer_parameters_invalid_device_and_xcode_combination_should_error() {
        let provided_os_version = Some(OsVersion::Ios17_5);
        let result = infer_parameters(None, provided_os_version).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_infer_parameters_valid_for_iphone_16_and_os_version_18_4() -> Result<()> {
        let provided_device = Some(IosDevice::IPhone16);
        let provided_os_version = Some(OsVersion::Ios18_4);

        let (inferred_device, inferred_os_version) =
            infer_parameters(provided_device, provided_os_version).await?;

        assert_eq!(inferred_device, IosDevice::IPhone16);
        assert_eq!(inferred_os_version, OsVersion::Ios18_4);

        Ok(())
    }
}
