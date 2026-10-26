use std::path::PathBuf;

use serde::Deserialize;

use crate::system::Settings;

pub const CONFIG_VARIABLE: &str = "FORGE_INSTALL_CONFIG";

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

impl Settings for Config {
    const VARIABLE: &'static str = CONFIG_VARIABLE;
    const UNSET_HINT: &'static str = "forge install runs from the installer ISO of a host";
}

impl Config {
    #[must_use]
    pub fn has_user(&self, user: &str) -> bool {
        self.users.iter().any(|known| known == user)
    }

    #[must_use]
    pub fn user_list(&self) -> String {
        self.users.join(" ")
    }
}
