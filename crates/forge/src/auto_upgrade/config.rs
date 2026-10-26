use std::path::PathBuf;

use serde::Deserialize;

use crate::system::Settings;

pub const CONFIG_VARIABLE: &str = "FORGE_AUTO_UPGRADE_CONFIG";

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

impl Settings for Config {
    const VARIABLE: &'static str = CONFIG_VARIABLE;
    const UNSET_HINT: &'static str = "this host does not import the auto-upgrade feature";
}

impl Config {
    #[must_use]
    pub fn branch(&self) -> String {
        format!("deploy/{}", self.host)
    }
}
