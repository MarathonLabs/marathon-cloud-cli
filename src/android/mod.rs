use std::fmt::Display;

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum Device {
    PHONE,
    TV,
    WEAR,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Device::PHONE => f.write_str("phone"),
            Device::TV => f.write_str("tv"),
            Device::WEAR => f.write_str("wear"),
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum SystemImage {
    #[clap(name = "default")]
    Default,
    #[clap(name = "google_apis")]
    GoogleApis,
}

impl Display for SystemImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemImage::Default => f.write_str("default"),
            SystemImage::GoogleApis => f.write_str("google_apis"),
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
}

impl Display for OsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsVersion::Android10 => f.write_str("10"),
            OsVersion::Android11 => f.write_str("11"),
            OsVersion::Android12 => f.write_str("12"),
            OsVersion::Android13 => f.write_str("13"),
            OsVersion::Android14 => f.write_str("14"),
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
