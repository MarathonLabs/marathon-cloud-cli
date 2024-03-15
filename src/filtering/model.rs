use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize, Serialize)]
pub struct SparseMarathonfile {
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
