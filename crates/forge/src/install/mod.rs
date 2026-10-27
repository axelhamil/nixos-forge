mod config;
mod disks;
mod secrets;
mod steps;

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub use config::{CONFIG_VARIABLE, Config};

use crate::system::interrupt::{self, Flag, SIGINT};
use crate::system::prompt::{Console, Terminal};
use crate::system::{
    Answer, Commands, Files, Invocation, Machine, Storage, config as settings, privilege,
};
use steps::install;

pub const SUBCOMMAND: &str = "install";

const USAGE_ERROR: u8 = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    pub disk: Option<String>,
    pub passphrase_file: Option<PathBuf>,
    pub password_files: Vec<(String, PathBuf)>,
    pub beside_windows: bool,
    pub yes: bool,
    pub reboot: bool,
}

impl Options {
    #[must_use]
    pub fn password_file(&self, user: &str) -> Option<&Path> {
        self.password_files
            .iter()
            .find(|(owner, _)| owner == user)
            .map(|(_, file)| file.as_path())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Run(Options),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("unknown argument {0}")]
    Unknown(String),
    #[error("{0}")]
    Missing(&'static str),
}

impl ParseError {
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Unknown(_) => USAGE_ERROR,
            Self::Missing(_) => 1,
        }
    }
}

pub fn parse(args: &[OsString]) -> Result<Parsed, ParseError> {
    let mut options = Options {
        reboot: true,
        ..Options::default()
    };
    let mut args = args.iter().map(|arg| arg.to_string_lossy().into_owned());
    while let Some(arg) = args.next() {
        let mut value = |missing| {
            args.next()
                .filter(|value| !value.is_empty())
                .ok_or(ParseError::Missing(missing))
        };
        match arg.as_str() {
            "--disk" => options.disk = Some(value("--disk needs a device")?),
            "--passphrase-file" => {
                options.passphrase_file =
                    Some(PathBuf::from(value("--passphrase-file needs a file")?));
            }
            "--password-file" => {
                let entry = value("--password-file needs USER=FILE")?;
                let (user, file) = entry.split_once('=').unwrap_or((&entry, &entry));
                let (user, file) = (user.to_owned(), PathBuf::from(file));
                options.password_files.retain(|(owner, _)| *owner != user);
                options.password_files.push((user, file));
            }
            "--beside-windows" => options.beside_windows = true,
            "--yes" => options.yes = true,
            "--no-reboot" => options.reboot = false,
            "-h" | "--help" => return Ok(Parsed::Help),
            _ => return Err(ParseError::Unknown(arg)),
        }
    }
    Ok(Parsed::Run(options))
}

#[must_use]
pub fn usage(host_name: &str) -> String {
    format!(
        "Usage: forge install [--disk DEVICE] [--beside-windows] [--passphrase-file FILE] [--password-file USER=FILE]... [--yes] [--no-reboot]

Installs {host_name} on a whole disk and erases everything on it, or beside Windows.
Every value not given on the command line is asked for, unless --yes is set.

  --disk DEVICE              disk to erase, such as /dev/nvme0n1
  --beside-windows           keep Windows on the disk: NixOS goes in its free space and shares its ESP (docs/how-to/dual-boot.md)
  --passphrase-file FILE     file holding the disk encryption passphrase
  --password-file USER=FILE  file holding the login password of USER
  --yes                      ask nothing: erase without confirmation, fail on a missing value, reboot at the end
  --no-reboot                stay on the installer once the install is done
"
    )
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Stop {
    #[error("{0}")]
    Refused(String),
    #[error("stopped with exit code {0}")]
    Quit(u8),
    #[error("stopped by signal {0}")]
    Interrupted(i32),
}

impl Stop {
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Refused(_) => 1,
            Self::Quit(code) => *code,
            Self::Interrupted(signal) => interrupt::exit_code(*signal),
        }
    }

    fn failed(answer: &Answer) -> Self {
        Self::Quit(answer.exit_code())
    }
}

#[must_use]
pub fn run(args: &[OsString], config_path: Option<&Path>) -> u8 {
    if !privilege::runs_as_root() {
        let error = privilege::reexec_as_root(SUBCOMMAND, args);
        eprintln!("forge install: cannot run sudo: {error}");
        return 1;
    }

    let config: Config = match settings::load(config_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("forge install: {error}");
            return 1;
        }
    };

    let options = match parse(args) {
        Ok(Parsed::Run(options)) => options,
        Ok(Parsed::Help) => {
            print!("{}", usage(&config.host_name));
            return 0;
        }
        Err(error @ ParseError::Missing(_)) => {
            eprintln!("forge install: {error}");
            return error.exit_code();
        }
        Err(error) => {
            eprint!("{}", usage(&config.host_name));
            return error.exit_code();
        }
    };

    let interrupt = match Flag::register() {
        Ok(flag) => flag,
        Err(error) => {
            eprintln!("forge install: cannot watch for interrupts: {error}");
            return 1;
        }
    };
    let mut terminal = Console::stdio(Some(interrupt.clone()));
    let mut stderr = io::stderr();
    let mut ports = Ports {
        commands: &Machine,
        files: &Machine,
        storage: &Machine,
        terminal: &mut terminal,
        interrupt: &interrupt,
        out: &mut stderr,
    };
    install(&config, &options, &mut ports)
}

pub struct Ports<'a> {
    pub commands: &'a dyn Commands,
    pub files: &'a dyn Files,
    pub storage: &'a dyn Storage,
    pub terminal: &'a mut dyn Terminal,
    pub interrupt: &'a Flag,
    pub out: &'a mut dyn Write,
}

pub(crate) struct Session<'p, 'a> {
    pub config: &'p Config,
    pub ports: &'p mut Ports<'a>,
}

impl Session<'_, '_> {
    pub fn say(&mut self, text: &str) {
        let _ = write!(self.ports.out, "\n{text}\n");
    }

    pub fn print(&mut self, text: &str) {
        let _ = write!(self.ports.out, "{text}");
    }

    pub fn checkpoint(&self) -> Result<(), Stop> {
        self.ports.interrupt.checkpoint().map_err(Stop::Interrupted)
    }

    pub fn run(&self, invocation: &Invocation) -> Result<Answer, Stop> {
        let answer = self.ports.commands.run(invocation);
        self.checkpoint()?;
        Ok(answer)
    }

    pub fn line(&mut self, prompt: &str) -> Result<String, Stop> {
        let answer = self.ports.terminal.line(prompt);
        self.answered(answer)
    }

    pub fn secret(&mut self, prompt: &str) -> Result<String, Stop> {
        let answer = self.ports.terminal.secret(prompt);
        self.answered(answer)
    }

    fn answered(&self, answer: io::Result<String>) -> Result<String, Stop> {
        self.checkpoint()?;
        answer.map_err(|error| match error.kind() {
            io::ErrorKind::Interrupted => Stop::Interrupted(SIGINT),
            io::ErrorKind::UnexpectedEof => Stop::Quit(1),
            _ => Stop::Refused(error.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Result<Parsed, ParseError> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        parse(&args)
    }

    #[test]
    fn every_flag_is_read_and_reboot_is_the_default() {
        let Ok(Parsed::Run(options)) = parsed(&[
            "--disk",
            "/dev/vda",
            "--passphrase-file",
            "/tmp/pass",
            "--password-file",
            "alice=/tmp/alice",
            "--beside-windows",
            "--yes",
        ]) else {
            panic!("the flags were refused");
        };

        assert_eq!(options.disk.as_deref(), Some("/dev/vda"));
        assert_eq!(
            options.passphrase_file.as_deref(),
            Some(Path::new("/tmp/pass"))
        );
        assert_eq!(
            options.password_file("alice"),
            Some(Path::new("/tmp/alice"))
        );
        assert!(options.beside_windows && options.yes && options.reboot);
    }

    #[test]
    fn a_password_file_given_twice_for_a_user_keeps_the_last() {
        let Ok(Parsed::Run(options)) = parsed(&[
            "--password-file",
            "alice=/tmp/a",
            "--password-file",
            "alice=/tmp/b",
            "--no-reboot",
        ]) else {
            panic!("the flags were refused");
        };

        assert_eq!(options.password_files.len(), 1);
        assert_eq!(options.password_file("alice"), Some(Path::new("/tmp/b")));
        assert!(!options.reboot);
    }

    #[test]
    fn a_password_file_without_an_equal_sign_names_both_user_and_file_like_the_shell() {
        let Ok(Parsed::Run(options)) = parsed(&["--password-file", "alice"]) else {
            panic!("the flags were refused");
        };

        assert_eq!(options.password_file("alice"), Some(Path::new("alice")));
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error_with_exit_code_2() {
        let error = parsed(&["--disk", "/dev/vda", "--force"]).unwrap_err();

        assert_eq!(error, ParseError::Unknown("--force".to_owned()));
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn a_flag_without_its_value_or_with_an_empty_one_exits_1_with_its_message() {
        assert_eq!(
            parsed(&["--disk"]).unwrap_err(),
            ParseError::Missing("--disk needs a device")
        );
        assert_eq!(
            parsed(&["--passphrase-file", ""]).unwrap_err(),
            ParseError::Missing("--passphrase-file needs a file")
        );
        assert_eq!(parsed(&["--password-file"]).unwrap_err().exit_code(), 1);
    }

    #[test]
    fn help_wins_over_the_flags_after_it_but_not_over_an_unknown_one_before() {
        assert_eq!(parsed(&["--yes", "-h", "--bogus"]), Ok(Parsed::Help));
        assert_eq!(
            parsed(&["--bogus", "--help"]),
            Err(ParseError::Unknown("--bogus".to_owned()))
        );
    }

    #[test]
    fn the_usage_names_the_host_and_the_forge_command() {
        let usage = usage("example");

        assert!(usage.starts_with("Usage: forge install [--disk DEVICE]"));
        assert!(usage.contains("Installs example on a whole disk"));
        assert!(usage.ends_with("once the install is done\n"));
    }

    #[test]
    fn an_interrupt_exits_130_and_a_stop_request_143() {
        assert_eq!(Stop::Interrupted(2).exit_code(), 130);
        assert_eq!(Stop::Interrupted(15).exit_code(), 143);
        assert_eq!(Stop::Interrupted(1).exit_code(), 129);
    }

    #[test]
    fn a_failed_step_exits_with_its_own_code() {
        assert_eq!(Stop::failed(&Answer::exited(4)).exit_code(), 4);
        assert_eq!(Stop::failed(&Answer::failed("killed")).exit_code(), 1);
    }
}
