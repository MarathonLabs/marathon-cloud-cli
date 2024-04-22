use std::ffi::OsStr;
use std::fmt::Display;

use anyhow::Result;
use tokio::fs::File;
use walkdir::WalkDir;

use crate::compression;
use crate::errors::InputError;

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
}

impl Display for OsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsVersion::Ios16_4 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-16-4"),
            OsVersion::Ios17_2 => f.write_str("com.apple.CoreSimulator.SimRuntime.iOS-17-2"),
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone, PartialEq, Eq)]
pub enum XcodeVersion {
    #[clap(name = "14.3.1")]
    Xcode14_3_1,
    #[clap(name = "15.2")]
    Xcode15_2,
}

impl Display for XcodeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XcodeVersion::Xcode14_3_1 => f.write_str("14.3.1"),
            XcodeVersion::Xcode15_2 => f.write_str("15.2"),
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
