use anyhow::Result;
use clap::CommandFactory;
use clap::{Args, Parser, Subcommand};
use std::{fmt::Display, path::PathBuf};

use crate::errors::{default_error_handler, ConfigurationError};
use crate::filtering;
use crate::interactor::{DownloadArtifactsInteractor, TriggerTestRunInteractor};
use crate::ios::{self, IosDevice, OsVersion, XcodeVersion};
use crate::{
    android::{self, Flavor, SystemImage},
    interactor::GetDeviceCatalogInteractor,
};

#[derive(Parser)]
#[command(
    name = "marathon-cloud",
    about = "Marathon Cloud command-line interface",
    long_about = None,
    author,
    version,
    about,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

impl Cli {
    fn get_supported_configs() -> Vec<(Option<IosDevice>, Option<XcodeVersion>, Option<OsVersion>)>
    {
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
        ]
    }

    async fn infer_parameters(
        device: Option<IosDevice>,
        xcode_version: Option<XcodeVersion>,
        os_version: Option<OsVersion>,
    ) -> (Option<IosDevice>, Option<XcodeVersion>, Option<OsVersion>) {
        let supported_configs = Self::get_supported_configs();
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

    pub async fn run() -> Result<()> {
        let cli = Cli::parse();
        simple_logger::SimpleLogger::new()
            .env()
            .with_level(cli.verbose.log_level_filter())
            .init()
            .unwrap();

        let result = match cli.command {
            Some(Commands::Run(args)) => {
                let run_cmd = args.command;
                match run_cmd {
                    RunCommands::Android {
                        application,
                        test_application,
                        os_version,
                        system_image,
                        device,
                        common,
                        api_args,
                        flavor,
                        instrumentation_arg,
                        retry_args,
                    } => {
                        match (device.as_deref(), &flavor, &system_image, &os_version) {
                            (
                                Some("watch"),
                                _,
                                Some(SystemImage::Default) | None,
                                Some(_) | None,
                            )
                            | (
                                Some("watch"),
                                _,
                                Some(_),
                                Some(android::OsVersion::Android10)
                                | Some(android::OsVersion::Android12)
                                | Some(android::OsVersion::Android14),
                            ) => {
                                return Err(ConfigurationError::UnsupportedRunConfiguration { message: "Android Watch only supports google-apis system image and os versions 11 and 13".into() }.into());
                            }
                            (Some("tv"), _, Some(SystemImage::Default), Some(_) | None) => {
                                return Err(ConfigurationError::UnsupportedRunConfiguration {
                                    message: "Android TV only supports google-apis system image"
                                        .into(),
                                }
                                .into());
                            }
                            (
                                Some("tv") | Some("watch"),
                                Some(Flavor::JsJestAppium)
                                | Some(Flavor::PythonRobotFrameworkAppium),
                                _,
                                _,
                            ) => {
                                return Err(ConfigurationError::UnsupportedRunConfiguration {
                                    message: "js-jest-appium and python-robotframework-appium only support 'phone' devices"
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

                        TriggerTestRunInteractor {}
                            .execute(
                                &api_args.base_url,
                                &api_args.api_key,
                                common.name,
                                common.link,
                                common.wait,
                                common.isolated,
                                common.ignore_test_failures,
                                common.code_coverage,
                                retry_args.retry_quota_test_uncompleted,
                                retry_args.retry_quota_test_preventive,
                                retry_args.retry_quota_test_reactive,
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
                                true,
                                instrumentation_arg,
                                None,
                            )
                            .await
                    }
                    RunCommands::iOS {
                        application,
                        test_application,
                        os_version,
                        device,
                        xcode_version,
                        common,
                        api_args,
                        xctestrun_env,
                        xctestrun_test_env,
                        xctestplan_filter_file,
                        xctestplan_target_name,
                        retry_args,
                    } => {
                        let supported_configs = Self::get_supported_configs();
                        let (device, xcode_version, os_version) =
                            Self::infer_parameters(device, xcode_version, os_version).await;

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
                        let application = ios::ensure_format(application).await?;
                        let test_application = ios::ensure_format(test_application).await?;
                        TriggerTestRunInteractor {}
                            .execute(
                                &api_args.base_url,
                                &api_args.api_key,
                                common.name,
                                common.link,
                                common.wait,
                                common.isolated,
                                common.ignore_test_failures,
                                common.code_coverage,
                                retry_args.retry_quota_test_uncompleted,
                                retry_args.retry_quota_test_preventive,
                                retry_args.retry_quota_test_reactive,
                                filtering_configuration,
                                &common.output,
                                Some(application),
                                test_application,
                                os_version.map(|x| x.to_string()),
                                None,
                                device.map(|x| x.to_string()),
                                xcode_version.map(|x| x.to_string()),
                                None,
                                "iOS".to_owned(),
                                true,
                                xctestrun_env,
                                xctestrun_test_env,
                            )
                            .await
                    }
                }
            }
            Some(Commands::Download(args)) => {
                let interactor = DownloadArtifactsInteractor {};
                let _ = interactor
                    .execute(
                        &args.api_args.base_url,
                        &args.api_args.api_key,
                        &args.id,
                        args.wait,
                        &args.output,
                    )
                    .await;
                Ok(true)
            }
            Some(Commands::Devices(args)) => {
                let run_cmd = args.command;
                let interactor = GetDeviceCatalogInteractor {};
                match run_cmd {
                    DevicesCommands::Android { api_args } => {
                        let _ = interactor
                            .execute(&api_args.base_url, &api_args.api_key, &Platform::Android)
                            .await;
                    }
                }
                Ok(true)
            }
            Some(Commands::Completions { shell }) => {
                let mut app = Self::command();
                let bin_name = app.get_name().to_string();
                clap_complete::generate(shell, &mut app, bin_name, &mut std::io::stdout());
                Ok(true)
            }
            None => Ok(true),
        };

        match result {
            Ok(true) => ::std::process::exit(0),
            Ok(false) => ::std::process::exit(1),
            Err(error) => {
                let stderr = std::io::stderr();
                default_error_handler(error.into(), &mut stderr.lock());
                ::std::process::exit(1);
            }
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Submit a test run")]
    Run(RunArgs),
    #[clap(about = "Get supported devices")]
    Devices(DevicesArgs),
    #[clap(about = "Download artifacts from a previous test run")]
    Download(DownloadArgs),
    #[clap(about = "Output shell completion code for the specified shell (bash, zsh, fish)")]
    Completions { shell: clap_complete::Shell },
}

#[derive(Debug, clap::Parser)]
#[command(args_conflicts_with_subcommands = true)]
struct RunArgs {
    #[command(subcommand)]
    command: RunCommands,
}
/// Options valid for any subcommand.
#[derive(Debug, Clone, clap::Args)]
struct CommonRunArgs {
    #[arg(short, long, help = "Output folder for test run results")]
    output: Option<PathBuf>,

    #[arg(long, help = "Run each test in isolation, i.e. isolated batching.")]
    isolated: Option<bool>,

    #[arg(
        long,
        help = "Test filters supplied as a YAML file following the schema at https://docs.marathonlabs.io/runner/configuration/filtering/#filtering-logic. For iOS see also https://docs.marathonlabs.io/runner/next/ios#test-plans"
    )]
    filter_file: Option<PathBuf>,

    #[arg(
        long,
        default_value_t = true,
        help = "Wait for test run to finish if true, exits after triggering a run if false"
    )]
    wait: bool,

    #[arg(
        long,
        help = "Name for run, for example it could be description of commit"
    )]
    name: Option<String>,

    #[arg(
        long,
        help = "Optional link, for example it could be a link to source control commit or CI run"
    )]
    link: Option<String>,

    #[arg(
        long,
        help = "When tests fail and this option is true then cli will exit with code 0. By default, cli will exit with code 1 in case of test failures and 0 for passing tests"
    )]
    ignore_test_failures: Option<bool>,

    #[arg(
        long,
        help = "Collect code coverage if true. Requires setup external to Marathon Cloud, e.g. build flags, jacoco jar added to classpath, etc"
    )]
    code_coverage: Option<bool>,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct DownloadArgs {
    #[arg(short, long, help = "Output folder for test run results")]
    output: PathBuf,

    #[arg(long, help = "Test run id")]
    id: String,

    #[arg(
        long,
        default_value_t = true,
        help = "Wait for test run to finish if true, exits immediately if false"
    )]
    wait: bool,

    #[command(flatten)]
    api_args: ApiArgs,
}

#[derive(Debug, clap::Parser)]
#[command(args_conflicts_with_subcommands = true)]
struct DevicesArgs {
    #[command(subcommand)]
    command: DevicesCommands,
}

#[derive(Debug, Subcommand)]
enum DevicesCommands {
    #[clap(about = "Print supported Android devices")]
    Android {
        #[command(flatten)]
        api_args: ApiArgs,
    },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ApiArgs {
    #[arg(long, env("MARATHON_CLOUD_API_KEY"), help = "Marathon Cloud API key")]
    api_key: String,

    #[arg(
        long,
        default_value = "https://cloud.marathonlabs.io/api/v1",
        help = "Base url for Marathon Cloud API"
    )]
    base_url: String,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct RetryArgs {
    #[arg(long, help = "Number of allowed uncompleted executions per test")]
    retry_quota_test_uncompleted: Option<u32>,
    #[arg(long, help = "Number of allowed preventive retries per test")]
    retry_quota_test_preventive: Option<u32>,
    #[arg(long, help = "Number of allowed reactive retries per test")]
    retry_quota_test_reactive: Option<u32>,
}

#[derive(Debug, Subcommand)]
enum RunCommands {
    #[clap(about = "Run tests for Android")]
    Android {
        #[arg(
            short,
            long,
            help = "application filepath, example: /home/user/workspace/sample.apk"
        )]
        application: Option<PathBuf>,

        #[arg(
            short,
            long,
            help = "test application filepath, example: /home/user/workspace/testSample.apk"
        )]
        test_application: PathBuf,

        #[arg(value_enum, long, help = "OS version")]
        os_version: Option<android::OsVersion>,

        #[arg(value_enum, long, help = "Runtime system image")]
        system_image: Option<android::SystemImage>,

        #[arg(
            value_enum,
            long,
            help = "Device type id. Use `marathon-cloud devices android` to get a list of supported devices"
        )]
        device: Option<String>,

        #[arg(value_enum, long, help = "Test flavor")]
        flavor: Option<android::Flavor>,

        #[command(flatten)]
        common: CommonRunArgs,

        #[command(flatten)]
        api_args: ApiArgs,

        #[command(flatten)]
        retry_args: RetryArgs,

        #[arg(long, help = "Instrumentation arguments, example: FOO=BAR")]
        instrumentation_arg: Option<Vec<String>>,
    },
    #[allow(non_camel_case_types)]
    #[command(name = "ios")]
    #[clap(about = "Run tests for iOS")]
    iOS {
        #[arg(
            short,
            long,
            help = "application filepath, example: /home/user/workspace/sample.zip"
        )]
        application: PathBuf,

        #[arg(
            short,
            long,
            help = "test application filepath, example: /home/user/workspace/sampleUITests-Runner.zip"
        )]
        test_application: PathBuf,

        #[arg(value_enum, long, help = "iOS runtime version")]
        os_version: Option<ios::OsVersion>,

        #[arg(value_enum, long, help = "Device type")]
        device: Option<ios::IosDevice>,

        #[arg(value_enum, long, help = "Xcode version")]
        xcode_version: Option<ios::XcodeVersion>,

        #[command(flatten)]
        common: CommonRunArgs,

        #[command(flatten)]
        api_args: ApiArgs,

        #[command(flatten)]
        retry_args: RetryArgs,

        #[arg(
            long,
            help = "xctestrun environment variable (EnvironmentVariables item), example FOO=BAR"
        )]
        xctestrun_env: Option<Vec<String>>,

        #[arg(
            long,
            help = "xctestrun testing environment variable (TestingEnvironmentVariables item), example FOO=BAR"
        )]
        xctestrun_test_env: Option<Vec<String>>,

        #[arg(long, help = "Test filters supplied as .xctestplan file")]
        xctestplan_filter_file: Option<PathBuf>,

        #[arg(long, help = "Target name to use for test filtering in .xctestplan")]
        xctestplan_target_name: Option<String>,
    },
}

#[derive(Debug)]
pub enum Platform {
    Android,
    #[allow(non_camel_case_types)]
    iOS,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Android => f.write_str("Android"),
            Platform::iOS => f.write_str("iOS"),
        }
    }
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
            Cli::infer_parameters(provided_device, None, None).await;

        assert_eq!(inferred_device, Some(IosDevice::IPhone14));
        assert_eq!(inferred_xcode_version, expected_xcode_version);
        assert_eq!(inferred_os_version, expected_os_version);
    }

    #[tokio::test]
    async fn test_infer_parameters_xcode_version_provided() {
        let provided_xcode_version = Some(XcodeVersion::Xcode15_2);
        let expected_device = Some(IosDevice::IPhone15);
        let expected_os_version = Some(OsVersion::Ios17_2);

        let (inferred_device, inferred_xcode_version, inferred_os_version) =
            Cli::infer_parameters(None, provided_xcode_version, None).await;

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
            Cli::infer_parameters(None, None, provided_os_version).await;

        assert_eq!(inferred_device, expected_device);
        assert_eq!(inferred_xcode_version, expected_xcode_version);
        assert_eq!(inferred_os_version, Some(OsVersion::Ios16_4));
    }

    #[tokio::test]
    async fn test_infer_parameters_device_and_xcode_version_provided() {
        let provided_device = Some(IosDevice::IPhone14);
        let provided_xcode_version = Some(XcodeVersion::Xcode14_3_1);
        let expected_os_version = Some(OsVersion::Ios16_4);

        let (inferred_device, inferred_xcode_version, inferred_os_version) = Cli::infer_parameters(
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
