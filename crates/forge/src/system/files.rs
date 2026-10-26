use std::io;
use std::path::{Path, PathBuf};

use super::Owner;

pub trait Files {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn read_link(&self, path: &Path) -> Option<PathBuf>;
    fn is_dir(&self, path: &Path) -> bool;
    fn list(&self, dir: &Path) -> Vec<String>;
    fn canonicalize(&self, path: &Path) -> PathBuf;
    fn owner(&self, path: &Path) -> Option<Owner>;
}
