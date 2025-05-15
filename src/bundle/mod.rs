use crate::{cli::model::LocalFileReference, errors::InputError};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ApplicationBundle {
    pub application: PathBuf,
    pub test_application: PathBuf,
}

#[derive(Debug)]
pub struct ApplicationBundleReference {
    pub application: LocalFileReference,
    pub test_application: LocalFileReference,
}

#[derive(Debug)]
pub struct LibraryBundleReference {
    pub test_application: LocalFileReference,
}

pub fn transform(input: &str) -> Result<ApplicationBundle, InputError> {
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() != 2 {
        return Err(InputError::InvalidApplicationBundle {
            bundle: input.to_owned(),
        });
    }

    let application = PathBuf::from(parts[0]);
    let test_application = PathBuf::from(parts[1]);

    let bundle = ApplicationBundle {
        application,
        test_application,
    };

    Ok(bundle)
}
