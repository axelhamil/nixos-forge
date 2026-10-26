use std::io;
use std::path::Path;

pub trait Sizes {
    fn available(&self, path: &Path) -> io::Result<u64>;
    fn file_size(&self, path: &Path) -> io::Result<u64>;
    fn tree_size(&self, path: &Path) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    pub size: u64,
    pub avail: u64,
}

pub fn space(path: &Path) -> io::Result<Space> {
    let stats = rustix::fs::statvfs(path)?;
    Ok(Space {
        size: stats.f_blocks.saturating_mul(stats.f_frsize),
        avail: stats.f_bavail.saturating_mul(stats.f_frsize),
    })
}
