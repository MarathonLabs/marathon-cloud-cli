use anyhow::Result;
use md5::{Digest, Md5};
use std::{io::BufRead, path::PathBuf};

use crate::{cli::model::LocalFileReference, errors::InputError};

pub(crate) async fn md5_optional(path: Option<PathBuf>) -> Result<Option<LocalFileReference>> {
    if let Some(path) = &path {
        let result = md5(path.to_path_buf());
        return Ok(Some(result.await?));
    }

    Ok(None)
}

// The file has to be a binary file, otherwise different OSes will produce different results
pub(crate) async fn md5(path: PathBuf) -> Result<LocalFileReference> {
    if !path.exists() {
        return Err(InputError::InvalidFileName { path: path.clone() })?;
    }

    let path = path.clone();
    let worker = tokio::task::spawn_blocking(move || {
        let mut hasher = Md5::new();

        let file = std::fs::File::open(&path).unwrap();
        let len = file.metadata().unwrap().len();
        let buf_len = len.min(1_000_000) as usize;

        let mut reader = std::io::BufReader::with_capacity(buf_len, file);
        loop {
            let part = reader.fill_buf().unwrap();
            if part.is_empty() {
                break;
            }
            hasher.update(part);
            let part_len = part.len();
            reader.consume(part_len);
        }
        let digest = hasher.finalize();

        LocalFileReference {
            path: path.to_path_buf(),
            md5: format!("{:x}", digest),
        }
    });

    Ok(worker.await?)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_valid() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("hashing")
            .join("example.gif");
        let result = md5(fixture.clone()).await?;
        assert_eq!(result.md5, "39993ccc6e0f49775c10aac512ade2a6");
        Ok(())
    }

    #[tokio::test]
    async fn test_missing() -> Result<()> {
        let result = md5_optional(None).await?;
        assert!(result.is_none());
        Ok(())
    }
}
