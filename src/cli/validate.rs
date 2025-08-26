use crate::compression;
use std::{ffi::OsStr, path::Path};
use walkdir::WalkDir;

use crate::{cli::RetryArgs, errors::InputError};
use anyhow::Result;
use tokio::fs::File;

pub(crate) fn retry_args(retry_args: RetryArgs) -> RetryArgs {
    if retry_args.no_retries {
        RetryArgs::new(Some(0), Some(0), Some(0))
    } else {
        retry_args
    }
}

pub(crate) fn result_file_args(result_file_args: &super::ResultFileArgs) -> Result<()> {
    if let Some(result_file) = &result_file_args.result_file {
        match result_file.extension().map(|f| f.to_str()) {
            //If no extension then treat as json
            Some(Some("json")) | Some(None) => Ok(()),
            Some(Some("yaml")) | Some(Some("yml")) => Ok(()),
            Some(Some(x)) => Err(InputError::InvalidFileExtension {
                extension: x.to_owned(),
                supported: "json,yaml,yml".to_owned(),
            }
            .into()),
            None => Err(InputError::NonUTF8Path {
                path: result_file.to_owned(),
            }
            .into()),
        }
    } else {
        Ok(())
    }
}

pub(crate) async fn ensure_format(
    path: &Path,
    supported_extensions_file: &[&str],
    supported_extensions_dir: &[&str],
) -> Result<std::path::PathBuf> {
    if path.is_file()
        && path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|ext| supported_extensions_file.contains(&ext))
    {
        Ok(path.to_path_buf())
    } else if path.is_dir()
        && (supported_extensions_dir.is_empty()
            || path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| supported_extensions_dir.contains(&ext)))
    {
        let dst = &path.with_extension("zip");
        let dst_file = File::create(dst).await?;

        let walkdir = WalkDir::new(path);
        let it = walkdir.into_iter();
        let prefix = &path
            .parent()
            .unwrap_or(path)
            .to_str()
            .ok_or(InputError::NonUTF8Path {
                path: path.to_path_buf(),
            })?;

        compression::zip_dir(&mut it.filter_map(|e| e.ok()), prefix, dst_file).await?;
        Ok(dst.to_owned())
    } else {
        Err(InputError::UnsupportedArtifact {
            path: path.to_path_buf(),
            supported_files: supported_extensions_file.join(","),
            supported_folders: supported_extensions_dir.join(","),
        }
        .into())
    }
}
