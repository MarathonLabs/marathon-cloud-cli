use log::debug;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;

use crate::errors::InputError;

pub(crate) fn validate_flow(test_application: &Path, flow: &str) -> Result<PathBuf> {
    let flow = Path::new(flow);
    let absolute_flow = if flow.is_absolute() {
        flow.to_path_buf()
    } else {
        test_application.join(flow)
    };
    // Flows are either regular files or a directory
    if !absolute_flow.exists() || (!absolute_flow.is_dir() && !absolute_flow.is_file()) {
        return Err(InputError::InvalidFileName {
            path: flow.to_path_buf(),
        }
        .into());
    }

    let canonical_test_application = test_application.canonicalize()?;
    debug!(
        "Removing {} from {}",
        canonical_test_application.to_string_lossy(),
        flow.to_string_lossy()
    );
    let relative_flow = flow.strip_prefix(canonical_test_application)?;

    Ok(relative_flow.to_path_buf())
}
