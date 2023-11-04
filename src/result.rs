use crate::errors::CliError;

pub type Result<T> = std::result::Result<T, CliError>;
