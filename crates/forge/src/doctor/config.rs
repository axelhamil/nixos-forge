use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const CONFIG_FILE: &str = "/etc/forge/doctor.json";
pub const CONFIG_VARIABLE: &str = "FORGE_DOCTOR_CONFIG";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no {}, this machine is not a nixos-forge host", path.display())]
    Missing { path: PathBuf },
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub secure_boot: bool,
    pub luks_device: Option<PathBuf>,
    pub btrbk_calendar: Option<String>,
    pub snapshot_dir: PathBuf,
    pub snapshot_subvolumes: Vec<String>,
    pub update_flake: Option<PathBuf>,
    pub auto_upgrade_state: Option<PathBuf>,
    pub deploy_signers: usize,
    pub wiped_root: bool,
    pub ephemeral_paths: Vec<PathBuf>,
}

impl Config {
    #[must_use]
    pub fn upgrade_state(&self) -> Option<PathBuf> {
        self.auto_upgrade_state
            .as_ref()
            .map(|dir| dir.join("state.json"))
    }

    #[must_use]
    pub fn upgrade_history(&self) -> Option<PathBuf> {
        self.auto_upgrade_state
            .as_ref()
            .map(|dir| dir.join("history.tsv"))
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = fs::read(path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                return ConfigError::Missing {
                    path: path.to_path_buf(),
                };
            }
            ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }
        })?;
        serde_json::from_slice(&text).map_err(|source| ConfigError::Invalid {
            path: path.to_path_buf(),
            source,
        })
    }
}
