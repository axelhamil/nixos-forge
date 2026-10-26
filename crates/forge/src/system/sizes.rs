use std::io;
use std::path::Path;

pub trait Sizes {
    fn available(&self, path: &Path) -> io::Result<u64>;
    fn file_size(&self, path: &Path) -> io::Result<u64>;
    fn tree_size(&self, path: &Path) -> u64;
}
