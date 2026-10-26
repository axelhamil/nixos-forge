use serde::Deserialize;

use crate::system::Settings;

pub const CONFIG_VARIABLE: &str = "FORGE_ENROLL_CONFIG";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub luks_device: Option<String>,
    pub volume_key_description: Option<String>,
    pub pcrs: Option<String>,
}

impl Settings for Config {
    const VARIABLE: &'static str = CONFIG_VARIABLE;
    const UNSET_HINT: &'static str = "run the forge this system installs";
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::system::{ConfigError, config};

    #[test]
    fn a_field_the_config_does_not_know_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("enroll.json");
        fs::write(
            &path,
            r#"{"luksDevice": null, "volumeKeyDescription": null, "pcrs": null, "slot": 31}"#,
        )
        .unwrap();

        let error = config::load::<Config>(Some(&path)).unwrap_err();

        assert!(matches!(error, ConfigError::Invalid { .. }));
    }
}
