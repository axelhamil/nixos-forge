use std::io;
use std::path::Path;
use std::time::Duration;

use super::Owner;

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
    pub env: Vec<(String, String)>,
    pub timeout: Option<Duration>,
    pub run_as: Option<Owner>,
    pub output: Output,
    pub input: Option<Vec<u8>>,
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
    pub fn input(mut self, bytes: &[u8]) -> Self {
        self.input = Some(bytes.to_vec());
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
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answer {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Answer {
    #[must_use]
    pub fn ok(stdout: &str) -> Self {
        Self {
            success: true,
            code: Some(0),
            stdout: stdout.to_owned(),
            stderr: String::new(),
        }
    }

    #[must_use]
    pub fn failed(stderr: &str) -> Self {
        Self {
            success: false,
            code: None,
            stdout: String::new(),
            stderr: stderr.to_owned(),
        }
    }

    #[must_use]
    pub fn exited(code: i32) -> Self {
        Self {
            success: code == 0,
            code: Some(code),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn line(&self) -> &str {
        self.stdout.trim_end_matches('\n')
    }
}

pub trait Commands {
    fn run(&self, invocation: &Invocation) -> Answer;
}

#[derive(Debug)]
pub struct Streamed {
    pub succeeded: bool,
    pub logged: io::Result<()>,
}

pub trait Streams {
    fn stream(&self, invocation: &Invocation, log: &Path) -> io::Result<Streamed>;
}
