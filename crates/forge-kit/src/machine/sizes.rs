use std::fs;
use std::io;
use std::path::Path;

use super::Machine;
use crate::os::space::space;
use crate::ports::sizes::Sizes;

impl Sizes for Machine {
    fn available(&self, path: &Path) -> io::Result<u64> {
        space(path).map(|space| space.avail)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        fs::metadata(path).map(|metadata| metadata.len())
    }

    fn tree_size(&self, path: &Path) -> u64 {
        walkdir::WalkDir::new(path)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.metadata().ok())
            .map(|metadata| metadata.len())
            .sum()
    }
}
