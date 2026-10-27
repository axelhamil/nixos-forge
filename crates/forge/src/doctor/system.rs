use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use crate::system::Space;
use crate::system::{Clock, Commands, Files};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub target: PathBuf,
    pub fstype: String,
}

pub trait Host {
    fn uid(&self) -> u32;
    fn sudo_user(&self) -> Option<String>;
    fn space(&self, path: &Path) -> io::Result<Space>;
    fn mounts(&self) -> io::Result<Vec<Mount>>;
}

pub struct Forks {
    pub dirs: Vec<PathBuf>,
    pub builtins: Vec<String>,
    pub timeout: Duration,
}

pub struct System<'a> {
    pub commands: &'a dyn Commands,
    pub files: &'a dyn Files,
    pub host: &'a dyn Host,
    pub clock: &'a dyn Clock,
    pub root: PathBuf,
    pub forks: Forks,
}
