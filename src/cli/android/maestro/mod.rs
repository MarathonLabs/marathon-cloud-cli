use crate::{
    cli::{
        android::{validate_device_configuration, OsVersion, SystemImage},
        maestro,
        model::LocalFileReference,
        validate, AnalyticsArgs, ApiArgs, CommonRunArgs, RetryArgs,
    },
    errors::InputError,
    filtering,
    formatter::{Formatter, StandardFormatter},
    hash,
    interactor::TriggerTestRunInteractor,
};

use futures::try_join;
use log::debug;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run(
    application: std::path::PathBuf,
    test_application: std::path::PathBuf,
    flows: Vec<String>,
    os_version: Option<OsVersion>,
    device: Option<String>,
    common: CommonRunArgs,
    api_args: ApiArgs,
    maestro_env: Option<Vec<String>>,
    retry_args: RetryArgs,
    analytics_args: AnalyticsArgs,
) -> Result<bool> {
    validate_device_configuration(
        &os_version,
        &Some(super::SystemImage::GoogleApis),
        &device,
        &Some(super::Flavor::Native),
    )?;

    let filter_file = common.filter_file.map(filtering::convert::convert);
    let filtering_configuration = match filter_file {
        Some(future) => Some(future.await?),
        None => None,
    };

    let retry_args = validate::retry_args(retry_args);
    validate::result_file_args(&common.result_file_args)?;

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

    let (application, test_application, flows) =
        validate(application, test_application, &flows).await?;
    if let Some(s) = spinner {
        s.finish_and_clear()
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
            Some(flows),
            os_version.map(|x| x.to_string()),
            Some(SystemImage::GoogleApis.to_string()),
            device,
            None,
            Some("maestro".to_owned()),
            "Android".to_owned(),
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

pub(crate) async fn validate(
    application: PathBuf,
    test_application: PathBuf,
    flows: &[String],
) -> Result<(LocalFileReference, LocalFileReference, Vec<String>)> {
    let mut validated_flows: Vec<String> = Vec::new();
    for flow in flows {
        debug!("Validating flow: {}", &flow);
        let validated_flow = maestro::validate_flow(&test_application, flow)?;
        debug!("Validated flow: {}", &validated_flow.to_string_lossy());
        validated_flows.push(validated_flow.to_string_lossy().to_string());
    }

    let application = validate::ensure_format(&application, &["apk"], &[], false).await?;
    let test_application = validate::ensure_format(&test_application, &[], &[], false).await?;

    let application = hash::md5(application);
    let test_application = hash::md5(test_application);

    let (application, test_application) = try_join!(application, test_application,)?;
    if application.md5 == test_application.md5 {
        return Err(InputError::DuplicatedApplicationBundle {
            app: application.path.clone(),
            test: test_application.path.clone(),
        })?;
    }

    Ok((application, test_application, validated_flows))
}
