use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::{fmt::Display, path::PathBuf};

use crate::interactor::{DownloadArtifactsInteractor, TriggerTestRunInteractor};

#[derive(Parser)]
#[command(name = "marathon-cloud")]
#[command(about = "Marathon Cloud command-line interface", long_about = None)]
#[command(author, version, about, long_about = None)]
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

        match cli.command {
            Some(Commands::Run(args)) => {
                let run_cmd = args.command.unwrap();
                match run_cmd {
                    RunCommands::Android {
                        application,
                        test_application,
                        os_version,
                        system_image,
                    } => {
                        TriggerTestRunInteractor {}
                            .execute(
                                &args.base_url,
                                &args.api_key,
                                args.wait,
                                args.isolated,
                                &args.output,
                                application,
                                test_application,
                                os_version,
                                system_image.map(|x| x.to_string()),
                                "Android".to_owned(),
                                true,
                            )
                            .await
                    }
                    RunCommands::iOS {
                        application,
                        test_application,
                    } => {
                        TriggerTestRunInteractor {}
                            .execute(
                                &args.base_url,
                                &args.api_key,
                                args.wait,
                                args.isolated,
                                &args.output,
                                Some(application),
                                test_application,
                                None,
                                None,
                                "iOS".to_owned(),
                                true,
                            )
                            .await?;
                        Ok(())
                    }
                }
            }
            Some(Commands::Download(args)) => {
                let interactor = DownloadArtifactsInteractor {};
                interactor
                    .execute(
                        &args.base_url,
                        &args.api_key,
                        &args.id,
                        args.wait,
                        &args.output,
                    )
                    .await?;
                Ok(())
            }
            None => Ok(()),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    Run(RunArgs),
    Download(DownloadArgs),
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct RunArgs {
    #[arg(short, long, help = "Output folder for test run results")]
    output: Option<PathBuf>,

    #[arg(long, env("MARATHON_CLOUD_API_KEY"), help = "Marathon Cloud API key")]
    api_key: String,

    #[arg(long, help = "Run each test in isolation, i.e. isolated batching.")]
    isolated: Option<bool>,

    #[arg(
        long,
        help = "Test filters supplied as a YAML file following the schema at https://docs.marathonlabs.io/runner/configuration/filtering/#filtering-logic. For iOS see also https://docs.marathonlabs.io/runner/next/ios#test-plans"
    )] filter_file: Option<PathBuf>,

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
        default_value = "https://cloud.marathonlabs.io/api/v1",
        help = "Base url for Marathon Cloud API"
    )]
    base_url: String,

    #[command(subcommand)]
    command: Option<RunCommands>,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct DownloadArgs {
    #[arg(short, long, help = "Output folder for test run results")]
    output: PathBuf,

    #[arg(long, env("MARATHON_CLOUD_API_KEY"), help = "Marathon Cloud API key")]
    api_key: String,

    #[arg(long, help = "Test run id")]
    id: String,

    #[arg(
        long,
        default_value_t = true,
        help = "Wait for test run to finish if true, exits immediately if false"
    )]
    wait: bool,

    #[arg(
        long,
        default_value = "https://cloud.marathonlabs.io/api/v1",
        help = "Base url for Marathon Cloud API"
    )]
    base_url: String,
}

#[derive(Debug, Subcommand)]
enum RunCommands {
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

        #[arg(long, help = "OS version [10, 11, 12, 13]")]
        os_version: Option<String>,

        #[arg(value_enum, long, help = "Runtime system image")]
        system_image: Option<AndroidSystemImage>,
    },
    #[command(name = "ios")]
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
    },
}

#[derive(Debug)]
pub enum Platform {
    Android,
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

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum AndroidSystemImage {
    Default,
    GoogleApis,
}

impl Display for AndroidSystemImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AndroidSystemImage::Default => f.write_str("default"),
            AndroidSystemImage::GoogleApis => f.write_str("google_apis"),
        }
    }
}
