use nom;
use nom::Err;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FOError {
    #[error("Pattern parsing error: {0}")]
    PatternError(String),
    #[error("config error: {0}")]
    ConfigError(#[from] serde_yaml::Error),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}
