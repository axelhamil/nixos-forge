use std::io;
use std::path::Path;

pub trait Storage {
    fn is_block_device(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
    fn link(&self, target: &Path, link: &Path) -> io::Result<()>;
    fn install_dir(&self, path: &Path, mode: u32) -> io::Result<()>;
    fn install_file(&self, path: &Path, bytes: &[u8], mode: u32) -> io::Result<()>;
    fn create_dirs(&self, path: &Path) -> io::Result<()>;
    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path);
    fn remove_dir(&self, path: &Path) -> io::Result<()>;
}
