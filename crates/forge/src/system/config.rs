use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

pub trait Settings: DeserializeOwned {
    const VARIABLE: &'static str;
    const UNSET_HINT: &'static str;
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{variable} is not set: {hint}")]
    Unset {
        variable: &'static str,
        hint: &'static str,
    },
    #[error("cannot read {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid {}: {source}", path.display())]
    Invalid {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub fn load<T: Settings>(path: Option<&Path>) -> Result<T, ConfigError> {
    let path = path.ok_or(ConfigError::Unset {
        variable: T::VARIABLE,
        hint: T::UNSET_HINT,
    })?;
    let text = fs::read(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&text).map_err(|source| ConfigError::Invalid {
        path: path.to_path_buf(),
        source,
    })
}
