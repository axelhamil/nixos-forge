use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const CONFIG_VARIABLE: &str = "FORGE_INSTALL_CONFIG";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{CONFIG_VARIABLE} is not set: forge install runs from the installer ISO of a host")]
    Unset,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub host_name: String,
    pub system: String,
    pub disko: String,
    pub beside_windows: String,
    pub dual_boot: bool,
    pub target_link: PathBuf,
    pub passphrase_file: PathBuf,
    pub luks_name: String,
    pub users: Vec<String>,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        let path = path.ok_or(ConfigError::Unset)?;
        let text = fs::read(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_slice(&text).map_err(|source| ConfigError::Invalid {
            path: path.to_path_buf(),
            source,
        })
    }

    #[must_use]
    pub fn has_user(&self, user: &str) -> bool {
        self.users.iter().any(|known| known == user)
    }

    #[must_use]
    pub fn user_list(&self) -> String {
        self.users.join(" ")
    }
}
