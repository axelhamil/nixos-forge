use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::Machine;
use crate::ports::accounts::{Accounts, Owner};
use crate::ports::files::Files;

impl Files for Machine {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_link(&self, path: &Path) -> Option<PathBuf> {
        fs::read_link(path).ok()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn list(&self, dir: &Path) -> Vec<String> {
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect()
    }

    fn canonicalize(&self, path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn owner(&self, path: &Path) -> Option<Owner> {
        let metadata = fs::metadata(path).ok()?;
        let uid = metadata.uid();
        let gid = self
            .directory()
            .primary_group(uid)
            .unwrap_or(metadata.gid());
        Some(Owner { uid, gid })
    }
}
