use std::path::PathBuf;

use serde::Deserialize;

use crate::system::Settings;

pub const SYNC_VARIABLE: &str = "FORGE_DUAL_BOOT_CONFIG";
pub const BESIDE_WINDOWS_VARIABLE: &str = "FORGE_BESIDE_WINDOWS_CONFIG";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncSettings {
    pub esp_mount: PathBuf,
}

impl Settings for SyncSettings {
    const VARIABLE: &'static str = SYNC_VARIABLE;
    const UNSET_HINT: &'static str = "run the forge of a system with the dual-boot feature";
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BesideWindowsSettings {
    pub format_mount: String,
    pub esp_device: PathBuf,
    pub luks_device: PathBuf,
}

impl Settings for BesideWindowsSettings {
    const VARIABLE: &'static str = BESIDE_WINDOWS_VARIABLE;
    const UNSET_HINT: &'static str = "run the forge of system.build.installBesideWindows";
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

    #[test]
    fn the_beside_windows_settings_are_read_in_camel_case() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("beside.json");
        fs::write(
            &path,
            r#"{"formatMount": "/nix/store/disko", "espDevice": "/dev/disk/by-partlabel/disk-main-ESP", "luksDevice": "/dev/disk/by-partlabel/disk-main-luks"}"#,
        )
        .unwrap();

        let settings = config::load::<BesideWindowsSettings>(Some(&path)).unwrap();

        assert_eq!(settings.format_mount, "/nix/store/disko");
        assert_eq!(
            settings.luks_device,
            Path::new("/dev/disk/by-partlabel/disk-main-luks")
        );
    }
}
