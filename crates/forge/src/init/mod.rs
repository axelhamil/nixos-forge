mod answers;
#[cfg(test)]
mod fake;
mod key;
mod options;
mod render;
mod steps;

use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

pub use options::{Flags, ParseError, Parsed, USAGE, parse};

use crate::system::interrupt::{self, Flag};
use crate::system::prompt::{Console, Terminal};
use crate::system::remote::UPSTREAM_REPOSITORY;
use crate::system::{Answer, Commands, Files, Invocation, Machine, Storage, find_in_path};

pub const SUBCOMMAND: &str = "init";

const USAGE_ERROR: u8 = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    pub upstream: String,
    pub runtime_dir: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub language: String,
    pub nix_on_path: bool,
    pub unique: String,
}

impl Environment {
    #[must_use]
    pub fn current() -> Self {
        let variable = |name| {
            env::var(name)
                .ok()
                .filter(|value: &String| !value.is_empty())
        };
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        Self {
            upstream: variable("UPSTREAM").unwrap_or_else(|| UPSTREAM_REPOSITORY.to_owned()),
            runtime_dir: variable("XDG_RUNTIME_DIR").map(PathBuf::from),
            temp_dir: env::temp_dir(),
            language: variable("LC_ALL")
                .or_else(|| variable("LANG"))
                .unwrap_or_default(),
            nix_on_path: find_in_path("nix", &env::var("PATH").unwrap_or_default()).is_some(),
            unique: format!("{}-{nanos}", process::id()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Stop {
    #[error("{0}")]
    Refused(String),
    #[error("stopped by signal {0}")]
    Interrupted(i32),
}

impl Stop {
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Refused(_) => 1,
            Self::Interrupted(signal) => interrupt::exit_code(*signal),
        }
    }
}

pub struct Ports<'a> {
    pub commands: &'a dyn Commands,
    pub files: &'a dyn Files,
    pub storage: &'a dyn Storage,
    pub terminal: &'a mut dyn Terminal,
    pub interrupt: &'a Flag,
    pub enter: &'a dyn Fn(&Path) -> io::Result<PathBuf>,
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

impl Ports<'_> {
    fn checkpoint(&self) -> Result<(), Stop> {
        self.interrupt.checkpoint().map_err(Stop::Interrupted)
    }

    fn run(&self, invocation: &Invocation) -> Result<Answer, Stop> {
        let answer = self.commands.run(invocation);
        self.checkpoint()?;
        Ok(answer)
    }

    fn warn(&mut self, text: &str) {
        let _ = writeln!(self.err, "{text}");
    }
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let flags = match parse(args) {
        Ok(Parsed::Run(flags)) => flags,
        Ok(Parsed::Help) => {
            print!("{USAGE}");
            return 0;
        }
        Err(error) => {
            eprintln!("forge init: {error}");
            return USAGE_ERROR;
        }
    };

    let interrupt = match Flag::register() {
        Ok(flag) => flag,
        Err(error) => {
            eprintln!("forge init: cannot watch for interrupts: {error}");
            return 1;
        }
    };
    let mut terminal = Console::stdio(Some(interrupt.clone()));
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let enter = |top: &Path| {
        env::set_current_dir(top)?;
        env::current_dir()
    };
    let mut ports = Ports {
        commands: &Machine,
        files: &Machine,
        storage: &Machine,
        terminal: &mut terminal,
        interrupt: &interrupt,
        enter: &enter,
        out: &mut stdout,
        err: &mut stderr,
    };
    steps::init(&flags, &Environment::current(), &mut ports)
}
