use std::fs::{self, DirBuilder, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, OpenOptionsExt, PermissionsExt, symlink};
use std::path::Path;

use super::Machine;
use crate::ports::storage::Storage;

impl Storage for Machine {
    fn is_block_device(&self, path: &Path) -> bool {
        fs::metadata(path).is_ok_and(|metadata| metadata.file_type().is_block_device())
    }

    fn exists(&self, path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok()
    }

    fn link(&self, target: &Path, link: &Path) -> io::Result<()> {
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_if_exists(link)?;
        symlink(target, link)
    }

    fn create_new_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        DirBuilder::new().mode(mode).create(path)
    }

    fn install_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        fs::create_dir_all(path)?;
        fs::set_permissions(path, Permissions::from_mode(mode))
    }

    fn install_file(&self, path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
        write_with_mode(path, bytes, mode)
    }

    fn create_dirs(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        fs::write(path, bytes)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove(&self, path: &Path) {
        let _ = remove_if_exists(path);
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }
}

fn write_with_mode(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)?;
    file.set_permissions(Permissions::from_mode(mode))?;
    file.write_all(bytes)
}

pub fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_dir_gets_its_mode_and_an_existing_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let made = dir.path().join("deploy-key");

        Machine.create_new_dir(&made, 0o700).unwrap();

        let permissions = fs::metadata(&made).unwrap().permissions();
        assert_eq!(permissions.mode() & 0o777, 0o700);
        assert_eq!(
            Machine
                .create_new_dir(&made, 0o700)
                .map_err(|error| error.kind()),
            Err(io::ErrorKind::AlreadyExists)
        );
    }
}
