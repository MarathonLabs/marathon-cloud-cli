mod android;
mod ios;
pub mod model;
mod validate;

use anyhow::Result;
use clap::CommandFactory;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::errors::default_error_handler;
use crate::interactor::{DownloadArtifactsInteractor, GetDeviceCatalogInteractor};

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
                        analytics_args,
                        pull_files,
                        application_bundle,
                        library_bundle,
                    } => {
                        android::run(
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
                            analytics_args,
                            pull_files,
                            application_bundle,
                            library_bundle,
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
                        analytics_args,
                        test_timeout_default,
                        test_timeout_max,
                        granted_permission,
                    } => {
                        ios::run(
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
                            analytics_args,
                            test_timeout_default,
                            test_timeout_max,
                            granted_permission,
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
                        args.glob,
                        args.progress_args.no_progress_bars,
                    )
                    .await;
                Ok(true)
            }
            Some(Commands::Devices(args)) => {
                let run_cmd = args.command;
                let interactor = GetDeviceCatalogInteractor {};
                match run_cmd {
                    DevicesCommands::Android {
                        api_args,
                        progress_args,
                    } => {
                        let _ = interactor
                            .execute(
                                &api_args.base_url,
                                &api_args.api_key,
                                &model::Platform::Android,
                                progress_args.no_progress_bars,
                            )
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
        help = "Test filters supplied as a YAML file following the schema at https://docs.marathonlabs.io/runner/configuration/filtering/#filtering-logic. 
For iOS see also https://docs.marathonlabs.io/runner/next/ios#test-plans.
Please be aware that if you use the 'annotation' filter type on Android, you should add the 'com.malinskiy.adam:android-junit4-test-annotation-producer:<version>' test dependency to parse custom test annotations."
    )]
    filter_file: Option<PathBuf>,

    #[arg(
        long,
        help = "Wait for test run to finish if true, exits after triggering a run if false"
    )]
    wait: Option<bool>,

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
        help = "Branch for run, for example it could be git branch like develop or feature/about-screen"
    )]
    branch: Option<String>,

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

    #[command(flatten)]
    progress_args: ProgressArgs,

    #[command(flatten)]
    result_file_args: ResultFileArgs,

    #[arg(
        long,
        help = "Limit maximum number of concurrent devices. 
Warning: Using this argument may BREAK the 15-minute run promise!"
    )]
    concurrency_limit: Option<u32>,

    #[arg(long, help = "The unique identifier (slug) for the project")]
    project: Option<String>,
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

    #[arg(
        long,
        help = "Only files matching this glob will be downloaded, i.e. 'tests/**' will download only the JUnit xml files"
    )]
    glob: Option<String>,

    #[command(flatten)]
    api_args: ApiArgs,

    #[command(flatten)]
    progress_args: ProgressArgs,

    #[command(flatten)]
    result_file_args: ResultFileArgs,
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
        #[command(flatten)]
        progress_args: ProgressArgs,
    },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ApiArgs {
    #[arg(long, env("MARATHON_CLOUD_API_KEY"), help = "Marathon Cloud API key")]
    api_key: String,

    #[arg(
        long,
        default_value = "https://cloud.marathonlabs.io/api",
        help = "Base url for Marathon Cloud API"
    )]
    base_url: String,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct RetryArgs {
    #[arg(
        long,
        conflicts_with = "no_retries",
        help = "Number of allowed uncompleted executions per test"
    )]
    retry_quota_test_uncompleted: Option<u32>,
    #[arg(
        long,
        conflicts_with = "no_retries",
        help = "Number of allowed preventive retries per test"
    )]
    retry_quota_test_preventive: Option<u32>,
    #[arg(
        long,
        conflicts_with = "no_retries",
        help = "Number of allowed reactive retries per test"
    )]
    retry_quota_test_reactive: Option<u32>,

    #[arg(long, default_value_t = false, help = "Disable all retries")]
    no_retries: bool,
}

impl RetryArgs {
    fn new(
        retry_quota_test_uncompleted: Option<u32>,
        retry_quota_test_preventive: Option<u32>,
        retry_quota_test_reactive: Option<u32>,
    ) -> Self {
        Self {
            retry_quota_test_uncompleted,
            retry_quota_test_preventive,
            retry_quota_test_reactive,
            no_retries: false,
        }
    }
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct AnalyticsArgs {
    #[arg(
        long,
        help = "If true then test run will not affect any statistical measurements"
    )]
    analytics_read_only: Option<bool>,
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true)]
struct ProgressArgs {
    #[arg(long, default_value_t = false, help = "Disable animated progress bars")]
    no_progress_bars: bool,
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true)]
struct ResultFileArgs {
    #[arg(
        long,
        help = "Result file path in a machine-readable format. You can specify the format via extension [yaml,json]"
    )]
    result_file: Option<PathBuf>,
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
        test_application: Option<PathBuf>,

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

        #[command(flatten)]
        analytics_args: AnalyticsArgs,

        #[arg(long, help = "Instrumentation arguments, example: FOO=BAR")]
        instrumentation_arg: Option<Vec<String>>,

        #[arg(
            long,
            help = "Pull files from devices after the test run. 
The format is 'ROOT:PATH' where ROOT is one of [EXTERNAL_STORAGE, APP_DATA] and PATH is a relative path to the target file or directory. 
Example: 'EXTERNAL_STORAGE:Documents/some-results', 'APP_DATA:files/my_folder/some_file.txt'. 
Note: Files with the same name and path from different devices may overwrite each other."
        )]
        pull_files: Option<Vec<String>>,

        #[arg(
            long,
            conflicts_with_all = &["application", "test_application"],
            help = "Application bundle containing the application apk and test application apk.
The format is '<app_apk_path>,<test_apk_path>'. The delimeter is a comma.
Example: '--application-bundle apks/feature1-app-debug.apk,apks/feature1-app-debug-androidTest.apk --application-bundle apks/feature2-app-debug.apk,apks/feature2-app-debug-androidTest.apk'"
        )]
        application_bundle: Option<Vec<String>>,

        #[arg(
            long,
            conflicts_with_all = &["application", "test_application"],
            help = "Library bundle containing the library test apk. Library testing requires only Test APK.
The format is '<test_apk_path>'.
Example: '--library-bundle apks/library1-debug-androidTest.apk --library-bundle apks/library2-debug-androidTest.apk'"
        )]
        library_bundle: Option<Vec<PathBuf>>,
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

        #[command(flatten)]
        analytics_args: AnalyticsArgs,

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

        #[arg(
            long,
            default_value = "300",
            help = "Default timeout for each test in seconds"
        )]
        test_timeout_default: Option<u32>,

        #[arg(
            long,
            help = "Maximum test timeout in seconds, overriding all other test timeout settings"
        )]
        test_timeout_max: Option<u32>,

        #[arg(
            long,
            help = "Grant permission to application.
Important: Granting is conducted before each test batch (not each test). If you need to grant before each test, please use --isolated mode.
Available permissions: calendar, contacts-limited, contacts, location, location-always, photos-add, photos, media-library, microphone, motion, reminders, siri."
        )]
        granted_permission: Option<Vec<String>>,
    },
}
