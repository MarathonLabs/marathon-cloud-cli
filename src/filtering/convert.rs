use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};

use crate::errors::{FilteringConfigurationError, InputError};

use super::{
    model::{Filter, FilteringConfiguration, SparseMarathonfile},
    xctestplan,
};

pub async fn convert(cnf: PathBuf) -> Result<SparseMarathonfile> {
    let content = fs::read_to_string(&cnf)
        .await
        .map_err(|error| InputError::OpenFileFailure {
            path: cnf.clone(),
            error,
        })?;

    let mut filtering_configuration: SparseMarathonfile = serde_yaml::from_str(&content)?;

    let absolute_path = fs::canonicalize(&cnf).await?;
    let workdir = absolute_path.parent().unwrap_or(Path::new(""));
    validate(
        &mut filtering_configuration.filtering_configuration,
        workdir,
    )
    .await?;

    Ok(filtering_configuration)
}

pub async fn convert_xctestplan(
    cnf: PathBuf,
    target_name: Option<String>,
) -> Result<SparseMarathonfile> {
    let content = fs::read_to_string(&cnf)
        .await
        .map_err(|error| InputError::OpenFileFailure {
            path: cnf.clone(),
            error,
        })?;

    let xctestplan: xctestplan::SparseTestPlan = serde_json::from_str(&content)?;
    let targets = xctestplan.test_targets;
    let target = match target_name {
        Some(target_name) => targets
            .iter()
            .find(|x| x.target.name == target_name)
            .ok_or(InputError::XctestplanMissingTargets)?,
        None => targets
            .first()
            .ok_or(InputError::XctestplanMissingTargets)?,
    };

    let allowlist = target
        .selected_tests
        .as_ref()
        .map(|x| xctestplan_ids_to_filter(x));
    let blocklist = target
        .skipped_tests
        .as_ref()
        .map(|x| xctestplan_ids_to_filter(x));

    let filtering_configuration = FilteringConfiguration {
        allowlist: allowlist.map(|x| vec![x]),
        blocklist: blocklist.map(|x| vec![x]),
    };
    let marathonfile = SparseMarathonfile {
        filtering_configuration,
    };

    Ok(marathonfile)
}

//Identifiers contain a mix of class names and class name with method signature
//Sometimes you can see separator \/ and sometimes / for the class and method
//Also sometime ending () are present for method filtering
fn xctestplan_ids_to_filter(ids: &[String]) -> Filter {
    let mut class_names: Vec<String> = vec![];
    let mut simple_test_names: Vec<String> = vec![];

    ids.iter().for_each(|id| {
        if id.contains("/") || id.contains("(") {
            let marathon_id = id.replace("\\/", "#").replace("/", "#").replace("()", "");
            simple_test_names.push(marathon_id);
        } else {
            class_names.push(id.clone());
        }
    });

    if !class_names.is_empty() && !simple_test_names.is_empty() {
        //Need to use composition since filtering is done via class names and also using methods
        let class_name_filter = Filter {
            mtype: "simple-class-name".into(),
            values: Some(class_names),
            op: None,
            file: None,
            regex: None,
            filters: None,
        };

        let simple_qualified_test_name_filter = Filter {
            mtype: "simple-test-name".into(),
            values: Some(simple_test_names),
            op: None,
            file: None,
            regex: None,
            filters: None,
        };
        Filter {
            mtype: "composition".into(),
            values: None,
            op: Some("UNION".into()),
            filters: Some(vec![class_name_filter, simple_qualified_test_name_filter]),
            regex: None,
            file: None,
        }
    } else if !class_names.is_empty() {
        Filter {
            mtype: "simple-class-name".into(),
            values: Some(class_names),
            op: None,
            file: None,
            regex: None,
            filters: None,
        }
    } else {
        Filter {
            mtype: "simple-test-name".into(),
            values: Some(simple_test_names),
            op: None,
            file: None,
            regex: None,
            filters: None,
        }
    }
}

pub async fn validate(cnf: &mut FilteringConfiguration, workdir: &Path) -> Result<()> {
    let supported_types = vec![
        "fully-qualified-class-name",
        "fully-qualified-test-name",
        "simple-class-name",
        "simple-test-name",
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
                            validate_filter(filter, supported_types, unsupported_types, workdir)
                                .await?;
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
        anyhow::bail!(FilteringConfigurationError::InvalidFilterType {
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::path::Path;

    use crate::filtering::convert::{convert, convert_xctestplan};

    #[tokio::test]
    async fn test_valid() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("valid.yaml");
        let result = convert(fixture).await?;
        let result = serde_json::to_string(&result)?;
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
        let result = serde_json::to_string(&result)?;

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
        let result = serde_json::to_string(&result)?;
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

    #[tokio::test]
    async fn test_xctestplan_1() -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture = Path::new(&manifest_dir)
            .join("fixture")
            .join("filtering")
            .join("xctestplan")
            .join("1.json");
        let result = convert_xctestplan(fixture, None).await?;
        let result = serde_json::to_string(&result)?;
        assert_eq!(
            result,
            r#"{"filteringConfiguration":{"blocklist":[{"type":"composition","filters":[{"type":"simple-class-name","values":["CrashingTests"]},{"type":"simple-test-name","values":["MoreTests#testDismissModal","SlowTests#testTextSlow3"]}],"op":"UNION"}]}}"#
        );
        Ok(())
    }
}
