use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const CONFIG_VARIABLE: &str = "FORGE_ENROLL_CONFIG";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{CONFIG_VARIABLE} is not set: run the forge this system installs")]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub luks_device: Option<String>,
    pub volume_key_description: Option<String>,
    pub pcrs: Option<String>,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_the_config_does_not_know_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("enroll.json");
        fs::write(
            &path,
            r#"{"luksDevice": null, "volumeKeyDescription": null, "pcrs": null, "slot": 31}"#,
        )
        .unwrap();

        let error = Config::load(Some(&path)).unwrap_err();

        assert!(matches!(error, ConfigError::Invalid { .. }));
    }
}
