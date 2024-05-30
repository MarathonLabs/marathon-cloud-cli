use crate::{cli::RetryArgs, errors::InputError};
use anyhow::Result;

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
