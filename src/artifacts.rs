use serde_json::Value;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::path::PathBuf;

use ::futures::{stream, StreamExt, TryStreamExt};
use anyhow::Result;
use indicatif::ProgressBar;
use log::debug;

use crate::api::{Artifact, RapiClient, RapiReqwestClient};
use crate::errors::ArtifactError;

pub async fn fetch_artifact_list(
    client: &RapiReqwestClient,
    id: &str,
    token: &str,
) -> Result<Vec<Artifact>> {
    let mut artifacts: Vec<Artifact> = Vec::new();
    let mut list: Vec<String> = vec![id.to_owned()];

    loop {
        let stats: Vec<Artifact> = stream::iter(list.clone().into_iter())
            .map(|dir| {
                let client = client.clone();
                let token = token.to_owned();
                tokio::spawn(async move { client.list_artifact(&token, &dir).await.unwrap() })
            })
            .buffer_unordered(num_cpus::get())
            .try_concat()
            .await
            .map_err(|error| ArtifactError::ListFailed { error })?;

        list.clear();
        for f in stats {
            if f.is_file {
                artifacts.push(f);
            } else {
                list.push(f.id);
            }
        }

        if list.is_empty() {
            break;
        }
    }

    Ok(artifacts)
}

pub async fn download_artifacts(
    client: &RapiReqwestClient,
    run_id: &str,
    artifacts: Vec<Artifact>,
    path: &PathBuf,
    token: &str,
    no_progress_bar: bool,
) -> Result<()> {
    debug!("Downloading {} artifacts:", artifacts.len());

    artifacts.iter().for_each(|f| debug!("{}", f.id));

    let mut progress_bar: Option<ProgressBar> = None;
    if !no_progress_bar {
        progress_bar = Some(ProgressBar::new(artifacts.len() as u64))
    }

    stream::iter(artifacts.into_iter())
        .map(|artifact| {
            let client = client.clone();
            let token = token.to_owned();
            let base_path = path.clone();
            let run_id = run_id.to_owned().clone();
            let progress_bar = progress_bar.clone();
            tokio::spawn(async move {
                for _try in 1..=3 {
                    let download_result = &client
                        .download_artifact(&token, artifact.clone(), base_path.clone(), &run_id)
                        .await;
                    match download_result {
                        Ok(_) => {
                            if let Some(progress_bar) = progress_bar {
                                progress_bar.inc(1);
                            }
                            return;
                        }
                        Err(error) => {
                            if _try < 4 {
                                debug!("Error fetching {}, retrying", artifact.id);
                                continue;
                            } else {
                                panic!(
                                    "Error fetching {}. All {} retries failed. {}",
                                    artifact.id, 3, error
                                );
                            }
                        }
                    }
                }
            })
        })
        .buffer_unordered(num_cpus::get())
        .try_collect()
        .await
        .map_err(|error| ArtifactError::DownloadFailed { error })?;

    if let Some(progress_bar) = progress_bar {
        progress_bar.finish_with_message("done");
    }
    Ok(())
}

pub async fn patch_allure_paths(output: &Path) -> Result<()> {
    // Define the required path
    let required_path = output.join("report/allure-results");

    // Check if the required path exists
    if !required_path.exists() {
        debug!("Directory {:?} does not exist", required_path);
        return Ok(());
    }

    // Iterate over each file in the required path
    match fs::read_dir(&required_path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Err(e) = patch_file(&path).await {
                            panic!("Failed to patch file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            panic!("Failed to read directory {:?}: {}", required_path, e);
        }
    }
    Ok(())
}

async fn patch_file(path: &Path) -> io::Result<()> {
    // Read the JSON file
    let mut file = File::open(&path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Parse the JSON
    let mut json_value: Value = serde_json::from_str(&content)?;

    // Patch the JSON
    if let Some(attachments) = json_value
        .get_mut("attachments")
        .and_then(|v| v.as_array_mut())
    {
        for attachment in attachments {
            if let Some(source) = attachment.get_mut("source") {
                if let Some(source_str) = source.as_str() {
                    // touch only logs and video
                    if let Some(index) = source_str
                        .find("logs/omni")
                        .or_else(|| source_str.find("video/omni"))
                    {
                        let new_path = format!("../../{}", &source_str[index..]);
                        *source = Value::String(new_path);
                    }
                }
            }
        }
    }

    // Write the patched JSON back to the file
    let mut file = File::create(&path)?;
    file.write_all(serde_json::to_string_pretty(&json_value)?.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use tempdir::TempDir;

    fn read_fixture(fixture_name: &str) -> String {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fixture_path = Path::new(&manifest_dir)
            .join("fixture")
            .join("patch_allure")
            .join(fixture_name);
        std::fs::read_to_string(fixture_path).expect("Failed to read fixture")
    }

    #[tokio::test]
    async fn test_patch_allure_paths_directory_does_not_exist() {
        let temp_dir = TempDir::new("test_patch_allure_paths").unwrap();
        let output_path = temp_dir.path().join("non_existing");

        let result = patch_allure_paths(&output_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_patch_allure_paths_no_json_files() {
        let temp_dir = TempDir::new("test_patch_allure_paths").unwrap();
        let allure_results_path = temp_dir.path().join("report/allure-results");
        fs::create_dir_all(&allure_results_path).unwrap();

        let result = patch_allure_paths(temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_patch_allure_paths_patch_json_files() {
        let temp_dir = TempDir::new("test_patch_allure_paths").unwrap();
        let allure_results_path = temp_dir.path().join("report/allure-results");
        fs::create_dir_all(&allure_results_path).unwrap();

        let original_json = read_fixture("original.json");
        let expected_json = read_fixture("expected.json");

        let json_file_path = allure_results_path.join("sample.json");
        let mut file = File::create(&json_file_path).unwrap();
        file.write_all(original_json.as_bytes()).unwrap();

        let result = patch_allure_paths(temp_dir.path()).await;
        assert!(result.is_ok());

        let mut file = File::open(&json_file_path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        let expected_json_value: Value = serde_json::from_str(&expected_json).unwrap();
        let result_json_value: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(result_json_value, expected_json_value);
    }

    #[tokio::test]
    async fn test_patch_allure_paths_no_patch_required() {
        let temp_dir = TempDir::new("test_patch_allure_paths").unwrap();
        let allure_results_path = temp_dir.path().join("report/allure-results");
        fs::create_dir_all(&allure_results_path).unwrap();

        let original_json = read_fixture("original_no_attachments.json");
        let expected_json = read_fixture("original_no_attachments.json");

        let json_file_path = allure_results_path.join("sample.json");
        let mut file = File::create(&json_file_path).unwrap();
        file.write_all(original_json.as_bytes()).unwrap();

        let result = patch_allure_paths(temp_dir.path()).await;
        assert!(result.is_ok());

        let mut file = File::open(&json_file_path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        let expected_json_value: Value = serde_json::from_str(&expected_json).unwrap();
        let result_json_value: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(result_json_value, expected_json_value);
    }
}
