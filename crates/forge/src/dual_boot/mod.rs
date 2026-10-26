mod config;
mod sync;

use std::ffi::OsString;
use std::io;
use std::path::Path;

pub use config::{SYNC_VARIABLE, SyncSettings};

use crate::system::mount::TempMounts;
use crate::system::{Machine, config as settings};

pub const SUBCOMMAND: &str = "dual-boot";

const USAGE_ERROR: u8 = 2;

const USAGE: &str = "Usage: forge dual-boot sync
       forge dual-boot install-beside-windows DISK SYSTEM

  sync                    copy the Windows boot manager of another disk onto this ESP and put NixOS then Windows first in the firmware boot order
  install-beside-windows  make room for SYSTEM beside Windows on DISK
";

#[must_use]
pub fn run(args: &[OsString], config_path: Option<&Path>) -> u8 {
    let words: Vec<String> = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["sync"] => sync_command(config_path),
        ["install-beside-windows", ..] => {
            eprintln!("forge dual-boot: install-beside-windows is not available yet");
            USAGE_ERROR
        }
        ["-h" | "--help"] => {
            print!("{USAGE}");
            0
        }
        _ => {
            eprint!("{USAGE}");
            USAGE_ERROR
        }
    }
}

fn sync_command(config_path: Option<&Path>) -> u8 {
    let settings: SyncSettings = match settings::load(config_path) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("forge dual-boot: {error}");
            return 0;
        }
    };
    let mounts = TempMounts::new(&Machine);
    let mut stderr = io::stderr();
    let mut ports = sync::Ports {
        commands: &Machine,
        files: &Machine,
        storage: &Machine,
        mounts: &mounts,
        out: &mut stderr,
    };
    sync::sync(&settings, &mut ports);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ran(args: &[&str]) -> u8 {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        run(&args, None)
    }

    #[test]
    fn sync_exits_0_even_without_its_config() {
        assert_eq!(ran(&["sync"]), 0);
    }

    #[test]
    fn an_unknown_or_missing_subcommand_is_a_usage_error() {
        assert_eq!(ran(&[]), 2);
        assert_eq!(ran(&["resync"]), 2);
        assert_eq!(ran(&["sync", "--now"]), 2);
        assert_eq!(ran(&["--help"]), 0);
    }
}
