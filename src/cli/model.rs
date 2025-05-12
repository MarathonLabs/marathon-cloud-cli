use std::{fmt::Display, path::PathBuf};

#[derive(Debug)]
pub enum Platform {
    Android,
    #[allow(non_camel_case_types)]
    iOS,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Android => f.write_str("Android"),
            Platform::iOS => f.write_str("iOS"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileReference {
    pub url: String,
    pub md5: String,
}

#[derive(Debug, Clone)]
pub struct LocalFileReference {
    pub path: PathBuf,
    pub md5: String,
}
