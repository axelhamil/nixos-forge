use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const CONFIG_VARIABLE: &str = "FORGE_AUTO_UPGRADE_CONFIG";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{CONFIG_VARIABLE} is not set: this host does not import the auto-upgrade feature")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Boot,
    Switch,
}

impl Operation {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::Switch => "switch",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub host: String,
    pub state_dir: PathBuf,
    pub checkout: Option<PathBuf>,
    pub sources: Vec<String>,
    pub operation: Operation,
    pub rebuild_command: String,
    pub allow_local_build: bool,
    pub allowed_signers: Option<PathBuf>,
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
    pub fn branch(&self) -> String {
        format!("deploy/{}", self.host)
    }
}
