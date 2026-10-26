use std::path::Path;

use super::Files;

const EFI: &str = "/sys/firmware/efi";
const EFI_GLOBAL: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBoot {
    NoUefi,
    SetupMode,
    Enforced,
    Disabled,
}

#[must_use]
pub fn secure_boot(files: &dyn Files) -> SecureBoot {
    if !files.is_dir(Path::new(EFI)) {
        return SecureBoot::NoUefi;
    }

    if variable(files, "SetupMode") == Some(1) {
        return SecureBoot::SetupMode;
    }
    if variable(files, "SecureBoot") == Some(1) {
        return SecureBoot::Enforced;
    }
    SecureBoot::Disabled
}

fn variable(files: &dyn Files, name: &str) -> Option<u8> {
    let path = format!("{EFI}/efivars/{name}-{EFI_GLOBAL}");
    let bytes = files.read(Path::new(&path)).ok()?;
    bytes.get(4).copied()
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::io;
    use std::path::PathBuf;

    use super::*;
    use crate::system::Owner;

    #[derive(Default)]
    struct Firmware {
        files: HashMap<PathBuf, Vec<u8>>,
        dirs: HashSet<PathBuf>,
    }

    impl Firmware {
        fn uefi() -> Self {
            let mut firmware = Self::default();
            firmware.dirs.insert(PathBuf::from(EFI));
            firmware
        }

        fn variable(mut self, name: &str, value: u8) -> Self {
            self.files.insert(
                PathBuf::from(format!("{EFI}/efivars/{name}-{EFI_GLOBAL}")),
                vec![7, 0, 0, 0, value],
            );
            self
        }
    }

    impl Files for Firmware {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }

        fn read_link(&self, _: &Path) -> Option<PathBuf> {
            None
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.dirs.contains(path)
        }

        fn list(&self, _: &Path) -> Vec<String> {
            Vec::new()
        }

        fn canonicalize(&self, path: &Path) -> PathBuf {
            path.to_path_buf()
        }

        fn owner(&self, _: &Path) -> Option<Owner> {
            None
        }
    }

    #[test]
    fn a_machine_without_the_efi_directory_has_no_uefi() {
        let firmware = Firmware::default().variable("SecureBoot", 1);

        assert_eq!(secure_boot(&firmware), SecureBoot::NoUefi);
    }

    #[test]
    fn setup_mode_wins_over_the_secure_boot_variable() {
        let firmware = Firmware::uefi()
            .variable("SetupMode", 1)
            .variable("SecureBoot", 1);

        assert_eq!(secure_boot(&firmware), SecureBoot::SetupMode);
    }

    #[test]
    fn secure_boot_set_in_user_mode_is_enforced() {
        let firmware = Firmware::uefi()
            .variable("SetupMode", 0)
            .variable("SecureBoot", 1);

        assert_eq!(secure_boot(&firmware), SecureBoot::Enforced);
    }

    #[test]
    fn secure_boot_cleared_or_unreadable_is_disabled() {
        let cleared = Firmware::uefi()
            .variable("SetupMode", 0)
            .variable("SecureBoot", 0);
        let truncated = Firmware::uefi().variable("SetupMode", 0);

        assert_eq!(secure_boot(&cleared), SecureBoot::Disabled);
        assert_eq!(secure_boot(&truncated), SecureBoot::Disabled);
    }
}
