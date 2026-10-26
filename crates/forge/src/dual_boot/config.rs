use std::path::PathBuf;

use serde::Deserialize;

use crate::system::Settings;

pub const SYNC_VARIABLE: &str = "FORGE_DUAL_BOOT_CONFIG";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncSettings {
    pub esp_mount: PathBuf,
}

impl Settings for SyncSettings {
    const VARIABLE: &'static str = SYNC_VARIABLE;
    const UNSET_HINT: &'static str = "run the forge of a system with the dual-boot feature";
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::system::{ConfigError, config};

    #[test]
    fn the_esp_mount_is_read_and_an_unknown_field_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good.json");
        let bad = dir.path().join("bad.json");
        fs::write(&good, r#"{"espMount": "/boot"}"#).unwrap();
        fs::write(&bad, r#"{"espMount": "/boot", "disk": "/dev/vda"}"#).unwrap();

        let settings = config::load::<SyncSettings>(Some(&good)).unwrap();

        assert_eq!(settings.esp_mount, Path::new("/boot"));
        assert!(matches!(
            config::load::<SyncSettings>(Some(&bad)),
            Err(ConfigError::Invalid { .. })
        ));
    }
}
