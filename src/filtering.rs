use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::errors::{FilteringConfigurationError, InputError};

pub async fn convert(cnf: PathBuf) -> Result<String> {
    let content = fs::read_to_string(&cnf)
        .await
        .map_err(|error| InputError::OpenFileFailure {
            path: cnf.clone(),
            error,
        })?;

    let mut filtering_configuration: Marathonfile = serde_yaml::from_str(&content)?;

    let absolute_path = fs::canonicalize(&cnf).await?;
    let workdir = absolute_path.parent().unwrap_or(Path::new(""));
    validate(
        &mut filtering_configuration.filtering_configuration,
        workdir,
    )
    .await?;

    let result = serde_json::to_string(&filtering_configuration)?;
    Ok(result)
}

pub async fn validate(cnf: &mut FilteringConfiguration, workdir: &Path) -> Result<()> {
    let supported_types = vec![
        "fully-qualified-class-name",
        "fully-qualified-test-name",
        "simple-class-name",
        "package",
        "method",
        "annotation",
    ];
    let unsupported_types = vec!["allure", "fragmentation", "annotationData"];

    for list in [&mut cnf.allowlist, &mut cnf.blocklist] {
        match list {
            Some(filters) => {
                validate_filters(filters, &supported_types, &unsupported_types, workdir).await?
            }
            None => continue,
        }
    }

    Ok(())
}

async fn validate_filters(
    filters: &mut [Filter],
    supported_types: &[&str],
    unsupported_types: &[&str],
    workdir: &Path,
) -> Result<()> {
    for filter in filters.iter_mut() {
        if filter.mtype == "composition" {
            if filter.op.is_none() {
                anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
                    mtype: filter.mtype.clone(),
                    message: "missing 'op' field".to_owned()
                });
            } else if filter.op.as_ref().is_some_and(|op| op.is_empty()) {
                anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
                    mtype: filter.mtype.clone(),
                    message: "empty 'op' field".to_owned()
                });
            } else {
                match filter.filters.as_mut() {
                    Some(filters) => {
                        for filter in filters.iter_mut() {
                            validate_filter(filter, supported_types, unsupported_types, workdir).await?;
                        }
                    }
                    None => {
                        anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
                            mtype: filter.mtype.clone(),
                            message: "missing composition filters".to_owned()
                        });
                    }
                }
            }
        } else {
            validate_filter(filter, supported_types, unsupported_types, workdir).await?;
        }
    }
    Ok(())
}

async fn validate_filter(
    filter: &mut Filter,
    supported_types: &[&str],
    unsupported_types: &[&str],
    workdir: &Path,
) -> Result<()> {
    if unsupported_types.iter().any(|&t| t == filter.mtype) {
        anyhow::bail!(FilteringConfigurationError::UnsupportedFilterType {
            mtype: filter.mtype.clone(),
        });
    } else if !supported_types.iter().any(|&t| t == filter.mtype) {
        anyhow::bail!(FilteringConfigurationError::UnsupportedFilterType {
            mtype: filter.mtype.clone(),
        });
    }

    match (&filter.regex, &filter.values, &filter.file) {
        (None, None, None) => {
            anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
                mtype: filter.mtype.clone(),
                message: "At least one of regex, values or file should be specified".into()
            })
        }

        (None, None, Some(path)) => {
            if !path.is_relative() {
                anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
                    mtype: filter.mtype.clone(),
                    message: "File should be specified relative to the filter file".into()
                })
            } else if !workdir.join(path).is_file() {
                anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
                    mtype: filter.mtype.clone(),
                    message: "File does not exist or is not a regular file".into()
                })
            } else {
                let mut values_file = File::open(workdir.join(path)).await?;
                let size = values_file.metadata().await?.len();
                if size == 0 {
                    anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
                        mtype: filter.mtype.clone(),
                        message: "File does not exist or is not a regular file".into()
                    })
                }

                let mut buffer = String::new();
                values_file.read_to_string(&mut buffer).await?;

                let mut values = Vec::new();
                for value in buffer.lines() {
                    values.push(value.to_owned());
                }
                filter.values = Some(values);
                filter.file = None;
            }
            Ok(())
        }
        (None, Some(_), None) => Ok(()),
        (Some(_), None, None) => Ok(()),

        _ => anyhow::bail!(FilteringConfigurationError::InvalidFilterConfiguration {
            mtype: filter.mtype.clone(),
            message: "only one of [regex, values, file] can be specified".into()
        }),
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize)]
pub struct Marathonfile {
    #[serde(rename = "filteringConfiguration")]
    pub filtering_configuration: FilteringConfiguration,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize)]
pub struct FilteringConfiguration {
    #[serde(rename = "allowlist")]
    pub allowlist: Option<Vec<Filter>>,
    #[serde(rename = "blocklist")]
    pub blocklist: Option<Vec<Filter>>,
}

// Very simplstic and flattened representation of https://github.com/MarathonLabs/marathon/blob/0.9.1/configuration/src/main/kotlin/com/malinskiy/marathon/config/FilteringConfiguration.kt
#[skip_serializing_none]
#[derive(Deserialize, Serialize)]
pub struct Filter {
    #[serde(rename = "type")]
    pub mtype: String,

    #[serde[rename = "regex"]]
    pub regex: Option<String>,
    #[serde[rename = "values"]]
    pub values: Option<Vec<String>>,
    #[serde[rename = "file"]]
    pub file: Option<PathBuf>,

    #[serde[rename = "filters"]]
    pub filters: Option<Vec<Filter>>,
    #[serde[rename = "op"]]
    pub op: Option<String>,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::path::Path;

    use crate::filtering::convert;

    #[tokio::test]
    async fn test_valid() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("valid.yaml");
        let result = convert(fixture).await?;
        assert_eq!(
            result,
            r#"{"filteringConfiguration":{"allowlist":[{"type":"fully-qualified-test-name","regex":".*Test"}]}}"#
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_valid_complex() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("validComplex.yaml");
        let result = convert(fixture).await?;

        assert_eq!(
            result,
            r#"{"filteringConfiguration":{"allowlist":[{"type":"package","values":["com.example.tests"]},{"type":"composition","filters":[{"type":"method","regex":"test.*"},{"type":"annotation","values":["com.example.MyAnnotation"]}],"op":"UNION"}],"blocklist":[{"type":"package","values":["com.example.tests2"]}]}}"#
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_type() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("unknownType.yaml");
        let result = convert(fixture).await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_composition_fields() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("invalidCompositionFields.yaml");
        let result = convert(fixture).await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("invalid.yaml");
        let result = convert(fixture).await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_grammar_error() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("grammarError.yaml");
        let result = convert(fixture).await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_fragmentation() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("fragmentation.yaml");
        let result = convert(fixture).await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_filetype() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("filetype.yaml");
        let result = convert(fixture).await?;
        assert_eq!(
            result,
            r#"{"filteringConfiguration":{"allowlist":[{"type":"fully-qualified-test-name","values":["com.malinskiy.adam.SimpleTest#test1","com.malinskiy.adam.SimpleTest#test2"]}]}}"#
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_correct_no_fields() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("correctTypeNoFields.yaml");
        let result = convert(fixture).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_correct_two_fields() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("correctTypeTwoFields.yaml");
        let result = convert(fixture).await;
        assert!(result.is_err());
        Ok(())
    }
}
