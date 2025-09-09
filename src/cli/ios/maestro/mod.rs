use std::time::Duration;

use crate::{
    cli::{
        self,
        ios::{validate_device_configuration, IosDevice, OsVersion, XcodeVersion},
        maestro, validate, AnalyticsArgs, ApiArgs, CommonRunArgs, RetryArgs,
    },
    errors::InputError,
    filtering,
    formatter::{Formatter, StandardFormatter},
    hash,
    interactor::TriggerTestRunInteractor,
};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run(
    application: std::path::PathBuf,
    test_application_arg: std::path::PathBuf,
    flows: Vec<String>,
    os_version: Option<OsVersion>,
    device: Option<IosDevice>,
    xcode_version: Option<XcodeVersion>,
    common: CommonRunArgs,
    api_args: ApiArgs,
    maestro_env: Option<Vec<String>>,
    retry_args: RetryArgs,
    analytics_args: AnalyticsArgs,
) -> Result<bool> {
    let (device, xcode_version, os_version) =
        match validate_device_configuration(os_version, device, xcode_version).await {
            Ok(value) => value,
            Err(value) => return value,
        };

    let filter_file = common.filter_file.map(filtering::convert::convert);
    let filtering_configuration = match filter_file {
        Some(future) => Some(future.await?),
        None => None,
    };

    let application =
        validate::ensure_format(&application, &["zip", "ipa"], &["app"], true).await?;
    let test_application = validate::ensure_format(&test_application_arg, &[], &[], false).await?;

    let mut validated_flows: Vec<String> = Vec::new();
    for flow in &flows {
        debug!("Validating flow: {}", &flow);
        let validated_flow = maestro::validate_flow(&test_application_arg, flow)?;
        debug!("Validated flow: {}", &validated_flow.to_string_lossy());
        validated_flows.push(validated_flow.to_string_lossy().to_string());
    }

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
            Some(validated_flows),
            os_version.map(|x| x.to_string()),
            None,
            device.map(|x| x.to_string()),
            xcode_version.map(|x| x.to_string()),
            Some("maestro".to_owned()),
            "iOS".to_owned(),
            common.progress_args.no_progress_bars,
            common.result_file_args.result_file,
            None,
            maestro_env,
            None,
            common.concurrency_limit,
            None,
            None,
            common.project,
            None,
            None,
            None,
            formatter,
        )
        .await
}
