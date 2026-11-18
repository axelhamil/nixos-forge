use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

use forge_kit::{Machine, Storage, remove_if_exists};

use super::tpm2::Host;

const TPM_DEVICE: &str = "/dev/tpmrm0";
const UNLOCK_KEY_BYTES: usize = 48;

pub struct TpmDevice;

impl Host for TpmDevice {
    fn has_tpm(&self) -> bool {
        fs::metadata(TPM_DEVICE).is_ok_and(|metadata| metadata.file_type().is_char_device())
    }

    fn write_unlock_key(&self, path: &Path) -> io::Result<()> {
        let mut random = [0; UNLOCK_KEY_BYTES];
        File::open("/dev/urandom")?.read_exact(&mut random)?;
        let key = random.iter().fold(String::new(), |mut key, byte| {
            let _ = write!(key, "{byte:02x}");
            key
        });
        Machine.install_file(path, key.as_bytes(), 0o600)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        remove_if_exists(path)
    }
}
