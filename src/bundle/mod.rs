use crate::errors::InputError;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ApplicationBundle {
    pub app_path: PathBuf,
    pub test_app_path: PathBuf,
}

pub fn transform_and_validate_bundle(
    input_bundle: Vec<String>,
) -> Result<Vec<ApplicationBundle>, InputError> {
    let mut bundles = Vec::new();

    for input in input_bundle {
        let parts: Vec<&str> = input.split(',').collect();
        if parts.len() != 2 {
            return Err(InputError::InvalidApplicationBundle { bundle: input });
        }

        let app_path = PathBuf::from(parts[0]);
        let test_app_path = PathBuf::from(parts[1]);

        if !app_path.exists() {
            return Err(InputError::InvalidFileName { path: app_path });
        }

        if !test_app_path.exists() {
            return Err(InputError::InvalidFileName {
                path: test_app_path,
            });
        }

        let bundle = ApplicationBundle {
            app_path,
            test_app_path,
        };
        bundles.push(bundle);
    }

    Ok(bundles)
}
