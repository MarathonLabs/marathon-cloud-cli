#![allow(dead_code)]
use serde::Deserialize;

//Version 1
#[derive(Deserialize)]
pub struct TestPlan {
    #[serde[rename = "configurations"]]
    pub configurations: Vec<Configuration>,
    #[serde[rename = "defaultOptions"]]
    pub default_options: Options,
    #[serde[rename = "testTargets"]]
    pub test_targets: Vec<TestTarget>,
    #[serde[rename = "version"]]
    pub version: u32,
}

#[derive(Deserialize)]
pub struct SparseTestPlan {
    #[serde[rename = "configurations"]]
    pub configurations: Vec<Configuration>,
    #[serde[rename = "testTargets"]]
    pub test_targets: Vec<TestTarget>,
}

#[derive(Deserialize)]
pub struct Configuration {
    #[serde[rename = "id"]]
    pub id: String,
    #[serde[rename = "name"]]
    pub name: String,
    #[serde[rename = "options"]]
    pub options: Options,
}

#[derive(Deserialize)]
pub struct Options {
    #[serde[rename = "environmentVariableEntries"]]
    pub environmnent_variables: Option<Vec<EnvironmentVariableEntry>>,
    #[serde[rename = "targetForVariableExpansion"]]
    pub target_for_variable_expansion: Option<Target>,
    #[serde[rename = "addressSanitizer"]]
    pub address_sanitizer: Option<AddressSanitizer>,
    #[serde[rename = "threadSanitizerEnabled"]]
    pub thread_sanitizer_enabled: Option<bool>,
    #[serde[rename = "undefinedBehaviorSanitizerEnabled"]]
    pub undefined_behavior_sanitizer_enabled: Option<bool>,
    #[serde[rename = "commandLineArgumentEntries"]]
    pub command_line_arguments: Option<Vec<CommandLineArgumentEntry>>,
    #[serde[rename = "language"]]
    pub language: Option<String>,
    #[serde[rename = "region"]]
    pub region: Option<String>,
    #[serde[rename = "locationScenario"]]
    pub location_scenario: Option<LocationScenario>,
    #[serde[rename = "testTimeoutsEnabled"]]
    pub test_timeouts_enabled: Option<bool>,
    #[serde[rename = "testRepetitionMode"]]
    pub test_repetition_mode: Option<TestRepetitionMode>,
    #[serde[rename = "testExecutionOrdering"]]
    pub test_execution_ordering: Option<TestExecutionOrdering>,
    #[serde[rename = "maximumTestRepetitions"]]
    pub maximum_test_repetitions: Option<u32>,
    #[serde[rename = "defaultTestExecutionTimeAllowance"]]
    pub default_test_execution_time_allowance: Option<u32>,
    #[serde[rename = "maximumTestExecutionTimeAllowance"]]
    pub maximum_test_execution_time_allowance: Option<u32>,
    #[serde[rename = "codeCoverage"]]
    pub code_coverage: Option<bool>,
    #[serde[rename = "uiTestingScreenshotsLifetime"]]
    pub ui_testing_screenshots_lifetime: Option<AttachmentLifetime>,

    #[serde[rename = "mainThreadCheckerEnabled"]]
    pub main_thread_checker_enabled: Option<bool>,
    #[serde[rename = "nsZombieEnabled"]]
    pub nszombie_enabled: Option<bool>,
    #[serde[rename = "guardMallocEnabled"]]
    pub guard_malloc_enabled: Option<bool>,
    #[serde[rename = "mallocGuardEdgesEnabled"]]
    pub malloc_guard_edges_enabled: Option<bool>,
    #[serde[rename = "mallocScribbleEnabled"]]
    pub malloc_scribble_enabled: Option<bool>,
    #[serde[rename = "mallocStackLoggingOptions"]]
    pub malloc_stack_logging: Option<MallocStackLoggingOptions>,

    #[serde[rename = "areLocalizationScreenshotsEnabled"]]
    pub are_localization_screenshots_enabled: Option<bool>,
    #[serde[rename = "diagnosticCollectionPolicy"]]
    pub diagnostic_collection_policy: Option<DiagnosticCollectionPolicy>,
    #[serde[rename = "preferredScreenCaptureFormat"]]
    pub preferred_screen_capture_format: Option<ScreenCaptureFormat>,
    #[serde[rename = "userAttachmentLifetime"]]
    pub user_attachment_lifetime: Option<AttachmentLifetime>,
}

#[derive(Deserialize)]
pub struct Target {
    #[serde[rename = "containerPath"]]
    pub container_path: String,
    #[serde[rename = "identifier"]]
    pub identifier: String,
    #[serde[rename = "name"]]
    pub name: String,
}

#[derive(Deserialize)]
pub struct CommandLineArgumentEntry {
    #[serde[rename = "argument"]]
    pub argument: String,
    #[serde[rename = "enabled"]]
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct EnvironmentVariableEntry {
    #[serde[rename = "key"]]
    pub key: String,
    #[serde[rename = "value"]]
    pub value: String,
    #[serde[rename = "enabled"]]
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct LocationScenario {
    #[serde[rename = "identifier"]]
    pub identifier: String,
    #[serde[rename = "referenceType"]]
    pub reference_type: Option<LocationReferenceType>,
}

#[derive(Deserialize)]
pub struct TestTarget {
    #[serde[rename = "parallelizable"]]
    pub parallelizable: Option<bool>,
    #[serde[rename = "skippedTests"]]
    pub skipped_tests: Option<Vec<String>>,
    #[serde[rename = "selectedTests"]]
    pub selected_tests: Option<Vec<String>>,
    #[serde[rename = "target"]]
    pub target: Target,
    #[serde[rename = "enabled"]]
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct SparseTestTarget {
    #[serde[rename = "skippedTests"]]
    pub skipped_tests: Option<Vec<String>>,
    #[serde[rename = "selectedTests"]]
    pub selected_tests: Option<Vec<String>>,
    #[serde[rename = "target"]]
    pub target: Target,
}

#[derive(Deserialize)]
pub struct AddressSanitizer {
    #[serde[rename = "detectStackUseAfterReturn"]]
    pub detect_stack_use_after_return: Option<bool>,
    #[serde[rename = "enabled"]]
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct MallocStackLoggingOptions {
    #[serde[rename = "loggingType"]]
    pub logging_type: Option<MallocStackLoggingType>,
}

//None is represented as null value
#[derive(Deserialize)]
pub enum MallocStackLoggingType {
    #[serde[rename = "liveAllocations"]]
    LiveAllocationsOnly,
    #[serde[rename = "allAllocations"]]
    AllAllocationsAndFreeHistory,
}

#[derive(Deserialize)]
pub enum TestExecutionOrdering {
    #[serde[rename = "random"]]
    Random,
    #[serde[rename = "alphabetical"]]
    Alphabetical,
}

#[derive(Deserialize)]
pub enum AttachmentLifetime {
    #[serde[rename = "keepAlways"]]
    OnAndKeepAll,
    #[serde[rename = "deleteOnSuccess"]]
    OnAndDeleteIfTestSucceeds,
    #[serde[rename = "keepNever"]]
    Off,
}

#[derive(Deserialize)]
pub enum TestRepetitionMode {
    #[serde[rename = "untilFailure"]]
    UntilFailure,
    #[serde[rename = "retryOnFailure"]]
    RetryOnFailure,
    #[serde[rename = "fixedIterations"]]
    UpUntilMaximumRepetitions,
    #[serde[rename = "none"]]
    None,
}

#[derive(Deserialize)]
pub enum ScreenCaptureFormat {
    #[serde[rename = "screenshot"]]
    Screenshot,
    #[serde[rename = "video"]]
    RetryOnFailure,
}

#[derive(Deserialize)]
pub enum DiagnosticCollectionPolicy {
    #[serde[rename = "xcodebuild"]]
    WhenTestingWithXcodebuild,
    #[serde[rename = "Never"]]
    Never,
    #[serde[rename = "Always"]]
    Always,
}

#[derive(Deserialize)]
pub enum LocationReferenceType {
    #[serde[rename = "built-in"]]
    BuiltIn,
}
