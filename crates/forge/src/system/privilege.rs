use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use crate::plugins;

#[must_use]
pub fn runs_as_root() -> bool {
    rustix::process::geteuid().is_root()
}

#[must_use]
pub fn reexec_as_root(subcommand: &str, args: &[OsString]) -> io::Error {
    let program = env::args_os()
        .next()
        .map_or_else(|| PathBuf::from("forge"), |argv0| resolve(&argv0));
    Command::new("sudo")
        .arg(program)
        .arg(subcommand)
        .args(args)
        .exec()
}

fn resolve(argv0: &OsStr) -> PathBuf {
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
