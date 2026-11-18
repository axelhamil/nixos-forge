use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::Duration;

use zeroize::Zeroizing;

use crate::ports::accounts::Owner;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Output {
    #[default]
    Captured,
    Shown,
    Discarded,
    Attached,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub os_args: Vec<OsString>,
    pub env: Vec<(String, String)>,
    pub timeout: Option<Duration>,
    pub run_as: Option<Owner>,
    pub output: Output,
    pub input: Option<Zeroizing<Vec<u8>>>,
    pub output_limit: Option<u64>,
    pub stdout_file: Option<PathBuf>,
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
    pub fn os_args<S: AsRef<OsStr>>(mut self, args: impl IntoIterator<Item = S>) -> Self {
        self.os_args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));
        self
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
    pub fn input(mut self, bytes: &[u8]) -> Self {
        self.input = Some(Zeroizing::new(bytes.to_vec()));
        self
    }

    #[must_use]
    pub fn output_limit(mut self, bytes: u64) -> Self {
        self.output_limit = Some(bytes);
        self
    }

    #[must_use]
    pub fn stdout_to(mut self, path: &Path) -> Self {
        self.stdout_file = Some(path.to_path_buf());
        self
    }

    #[must_use]
    pub fn discard_output(mut self) -> Self {
        self.output = Output::Discarded;
        self
    }

    #[must_use]
    pub fn loud(mut self) -> Self {
        self.output = Output::Shown;
        self
    }

    #[must_use]
    pub fn attached(mut self) -> Self {
        self.output = Output::Attached;
        self
    }

    #[must_use]
    pub fn command_line(&self) -> String {
        std::iter::once(self.program.as_str().into())
            .chain(self.args.iter().map(|arg| arg.as_str().into()))
            .chain(self.os_args.iter().map(|arg| arg.to_string_lossy()))
            .collect::<Vec<Cow<'_, str>>>()
            .join(" ")
    }
}
