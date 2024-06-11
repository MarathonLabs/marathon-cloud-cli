use crate::errors::{self, PullArgError};
use serde::{Deserialize, Serialize};

const AGGREGATION_MODE_TEST_RUN: &str = "TEST_RUN";

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PullFileConfig {
    #[serde(rename = "pull")]
    pub pull_items: Vec<PullFileItem>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PullFileItem {
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    #[serde(rename = "aggregationMode")]
    pub aggregation_mode: String,
    #[serde(rename = "pathRoot")]
    pub path_root: String,
}

pub fn parse_pull_args(pull_args: Vec<String>) -> Result<PullFileConfig, errors::PullArgError> {
    let mut pulls = Vec::new();
    for arg in pull_args {
        let parts: Vec<&str> = arg.split(':').collect();
        if parts.len() != 2 {
            return Err(PullArgError::InvalidFormat {
                arg: arg.to_string(),
            });
        }
        let root = match parts[0] {
            "EXTERNAL_STORAGE" => "EXTERNAL_STORAGE",
            "APP_DATA" => "APP_DATA",
            _ => {
                return Err(PullArgError::InvalidRootType {
                    used_type: parts[0].to_string(),
                })
            }
        };
        let relative_path = parts[1].to_string();
        pulls.push(PullFileItem {
            relative_path,
            aggregation_mode: AGGREGATION_MODE_TEST_RUN.to_string(),
            path_root: root.to_string(),
        });
    }
    let pull_file_config = PullFileConfig { pull_items: pulls };
    Ok(pull_file_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pull_args() {
        let pull_args = vec![
            "EXTERNAL_STORAGE:my-device-folder1".to_string(),
            "APP_DATA:my-device-folder2/some_file.txt".to_string(),
        ];
        let result = parse_pull_args(pull_args);

        assert!(result.is_ok());
        let pull_file_config = result.unwrap();
        let pulls = pull_file_config.pull_items;
        assert_eq!(pulls.len(), 2);

        assert_eq!(
            pulls[0],
            PullFileItem {
                relative_path: "my-device-folder1".to_string(),
                aggregation_mode: AGGREGATION_MODE_TEST_RUN.to_string(),
                path_root: "EXTERNAL_STORAGE".to_string(),
            }
        );

        assert_eq!(
            pulls[1],
            PullFileItem {
                relative_path: "my-device-folder2/some_file.txt".to_string(),
                aggregation_mode: AGGREGATION_MODE_TEST_RUN.to_string(),
                path_root: "APP_DATA".to_string(),
            }
        );
    }

    #[test]
    fn test_invalid_format_pull_arg() {
        let pull_args = vec![
            "EXTERNAL_STORAGE:my-device-folder1".to_string(),
            "INVALID_FORMAT".to_string(),
        ];
        let result = parse_pull_args(pull_args);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            error,
            PullArgError::InvalidFormat {
                arg: "INVALID_FORMAT".to_string()
            }
        );
    }

    #[test]
    fn test_invalid_root_type_pull_arg() {
        let pull_args = vec![
            "EXTERNAL_STORAGE:my-device-folder1".to_string(),
            "UNKNOWN:my-device-folder2".to_string(),
        ];
        let result = parse_pull_args(pull_args);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            error,
            PullArgError::InvalidRootType {
                used_type: "UNKNOWN".to_string()
            }
        );
    }

    #[test]
    fn test_empty_pull_args() {
        let pull_args: Vec<String> = Vec::new();
        let result = parse_pull_args(pull_args);

        assert!(result.is_ok());
        let pull_file_config = result.unwrap();
        let pulls = pull_file_config.pull_items;
        assert_eq!(pulls.len(), 0);
    }
}
