mod config;
mod disks;
mod secrets;
mod steps;

use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt, symlink};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use config::{CONFIG_VARIABLE, Config, ConfigError};
pub use steps::install;

use crate::plugins;
use crate::system::interrupt::Flag;
use crate::system::prompt::Terminal;
use crate::system::{Answer, Commands, Files, Invocation, Machine};

pub const SUBCOMMAND: &str = "install";

const SIGINT: i32 = 2;
const INTERRUPTED: u8 = 130;
const TERMINATED: u8 = 143;
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
            Self::Interrupted(SIGINT) => INTERRUPTED,
            Self::Interrupted(_) => TERMINATED,
        }
    }

    fn failed(answer: &Answer) -> Self {
        let code = answer
            .code
            .and_then(|code| u8::try_from(code).ok())
            .filter(|&code| code != 0)
            .unwrap_or(1);
        Self::Quit(code)
    }
}

pub trait Storage {
    fn is_block_device(&self, path: &Path) -> bool;
    fn link(&self, target: &Path, link: &Path) -> io::Result<()>;
    fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn install_dir(&self, path: &Path, mode: u32) -> io::Result<()>;
    fn install_file(&self, path: &Path, bytes: &[u8], mode: u32) -> io::Result<()>;
    fn remove(&self, path: &Path);
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
        match self.ports.interrupt.raised() {
            Some(signal) => Err(Stop::Interrupted(signal)),
            None => Ok(()),
        }
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

#[must_use]
pub fn runs_as_root() -> bool {
    rustix::process::geteuid().is_root()
}

#[must_use]
pub fn reexec_as_root(args: &[OsString]) -> io::Error {
    let program = env::args_os()
        .next()
        .map(|argv0| resolve(&argv0))
        .or_else(|| env::current_exe().ok())
        .unwrap_or_else(|| PathBuf::from("forge"));
    Command::new("sudo")
        .arg(program)
        .arg(SUBCOMMAND)
        .args(args)
        .exec()
}

fn resolve(argv0: &OsString) -> PathBuf {
    let program = PathBuf::from(argv0);
    if program.components().count() > 1 {
        return program;
    }
    env::var_os("PATH")
        .and_then(|path| {
            env::split_paths(&path)
                .map(|dir| dir.join(&program))
                .find(|candidate| plugins::is_executable(candidate))
        })
        .unwrap_or(program)
}

impl Storage for Machine {
    fn is_block_device(&self, path: &Path) -> bool {
        fs::metadata(path).is_ok_and(|metadata| metadata.file_type().is_block_device())
    }

    fn link(&self, target: &Path, link: &Path) -> io::Result<()> {
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::remove_file(link) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        symlink(target, link)
    }

    fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?
            .write_all(bytes)
    }

    fn install_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        fs::create_dir_all(path)?;
        fs::set_permissions(path, Permissions::from_mode(mode))
    }

    fn install_file(&self, path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(mode)
            .open(path)?;
        file.set_permissions(Permissions::from_mode(mode))?;
        file.write_all(bytes)
    }

    fn remove(&self, path: &Path) {
        let _ = fs::remove_file(path);
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
        assert_eq!(Stop::Interrupted(1).exit_code(), 143);
    }

    #[test]
    fn a_failed_step_exits_with_its_own_code() {
        assert_eq!(Stop::failed(&Answer::exited(4)).exit_code(), 4);
        assert_eq!(Stop::failed(&Answer::failed("killed")).exit_code(), 1);
    }
}
