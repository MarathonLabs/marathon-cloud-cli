use anyhow::Result;
use clap::CommandFactory;
use clap::{Args, Parser, Subcommand};
use std::{fmt::Display, path::PathBuf};

use crate::android::{self, Device, SystemImage};
use crate::errors::{default_error_handler, ConfigurationError};
use crate::interactor::{DownloadArtifactsInteractor, TriggerTestRunInteractor};

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
    pub async fn run() -> Result<()> {
        let cli = Cli::parse();
        simple_logger::SimpleLogger::new()
            .env()
            .with_level(cli.verbose.log_level_filter())
            .init()
            .unwrap();

        let result = match cli.command {
            Some(Commands::Run(args)) => {
                let run_cmd = args.command.unwrap();
                match run_cmd {
                    RunCommands::Android {
                        application,
                        test_application,
                        os_version,
                        system_image,
                        device,
                        common,
                        api_args,
                    } => {
                        match (&device, &system_image, &os_version) {
                            (Some(Device::WEAR), Some(SystemImage::GoogleApis), Some(_) | None) => {
                                return Err(ConfigurationError::UnsupportedRunConfiguration { message: "Android Wear only supports default system image and os versions 11 and 13".into() }.into());
                            }
                            (
                                Some(Device::WEAR),
                                Some(_) | None,
                                Some(android::OsVersion::Android10)
                                | Some(android::OsVersion::Android12)
                                | Some(android::OsVersion::Android14),
                            ) => {
                                return Err(ConfigurationError::UnsupportedRunConfiguration { message: "Android Wear only supports default system image and os versions 11 and 13".into() }.into());
                            }
                            (Some(Device::TV), Some(SystemImage::GoogleApis), Some(_) | None) => {
                                return Err(ConfigurationError::UnsupportedRunConfiguration {
                                    message: "Android TV only supports default system image".into(),
                                }
                                .into());
                            }
                            _ => {}
                        }

                        TriggerTestRunInteractor {}
                            .execute(
                                &api_args.base_url,
                                &api_args.api_key,
                                common.wait,
                                common.isolated,
                                common.ignore_test_failures,
                                common.filter_file,
                                &common.output,
                                application,
                                test_application,
                                os_version.map(|x| x.to_string()),
                                system_image.map(|x| x.to_string()),
                                device.map(|x| x.to_string()),
                                "Android".to_owned(),
                                true,
                            )
                            .await
                    }
                    RunCommands::iOS {
                        application,
                        test_application,
                        common,
                        api_args,
                    } => {
                        TriggerTestRunInteractor {}
                            .execute(
                                &api_args.base_url,
                                &api_args.api_key,
                                common.wait,
                                common.isolated,
                                common.ignore_test_failures,
                                common.filter_file,
                                &common.output,
                                Some(application),
                                test_application,
                                None,
                                None,
                                None,
                                "iOS".to_owned(),
                                true,
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
    #[clap(about = "Download artifacts from a previous test run")]
    Download(DownloadArgs),
    #[clap(about = "Output shell completion code for the specified shell (bash, zsh, fish)")]
    Completions { shell: clap_complete::Shell },
}

#[derive(Debug, clap::Parser)]
#[command(args_conflicts_with_subcommands = true)]
struct RunArgs {
    #[command(subcommand)]
    command: Option<RunCommands>,
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
        help = "name for run, for example it could be description of commit"
    )]
    name: Option<String>,

    #[arg(long, help = "link to commit")]
    link: Option<String>,

    #[arg(
        long,
        help = "When tests fail and this option is true then cli will exit with code 0. By default, cli will exit with code 1 in case of test failures and 0 for passing tests"
    )]
    ignore_test_failures: Option<bool>,
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

        #[arg(value_enum, long, help = "Device type")]
        device: Option<android::Device>,

        #[command(flatten)]
        common: CommonRunArgs,

        #[command(flatten)]
        api_args: ApiArgs,
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

        #[command(flatten)]
        common: CommonRunArgs,

        #[command(flatten)]
        api_args: ApiArgs,
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
