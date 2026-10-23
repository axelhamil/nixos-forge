use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::TimeZone;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub timeout: Option<Duration>,
    pub run_as: Option<Owner>,
    pub discard_output: bool,
}

impl Invocation {
    #[must_use]
    pub fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_owned(),
            args: args.iter().map(|&arg| arg.to_owned()).collect(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn env(mut self, name: &str, value: &str) -> Self {
        self.env.push((name.to_owned(), value.to_owned()));
        self
    }

    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    #[must_use]
    pub fn run_as(mut self, owner: Owner) -> Self {
        self.run_as = Some(owner);
        self
    }

    #[must_use]
    pub fn discard_output(mut self) -> Self {
        self.discard_output = true;
        self
    }

    #[must_use]
    pub fn command_line(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answer {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl Answer {
    #[must_use]
    pub fn ok(stdout: &str) -> Self {
        Self {
            success: true,
            stdout: stdout.to_owned(),
            stderr: String::new(),
        }
    }

    #[must_use]
    pub fn failed(stderr: &str) -> Self {
        Self {
            success: false,
            stdout: String::new(),
            stderr: stderr.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Owner {
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    pub size: u64,
    pub avail: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub target: PathBuf,
    pub fstype: String,
}

pub trait Commands {
    fn run(&self, invocation: &Invocation) -> Answer;
}

pub trait Files {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn is_dir(&self, path: &Path) -> bool;
    fn list(&self, dir: &Path) -> Vec<String>;
    fn canonicalize(&self, path: &Path) -> PathBuf;
    fn owner(&self, path: &Path) -> Option<Owner>;
}

pub trait Host {
    fn uid(&self) -> u32;
    fn sudo_user(&self) -> Option<String>;
    fn space(&self, path: &Path) -> io::Result<Space>;
    fn mounts(&self) -> io::Result<Vec<Mount>>;
}

pub trait Clock {
    fn now(&self) -> Timestamp;
    fn zone(&self) -> TimeZone;
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
