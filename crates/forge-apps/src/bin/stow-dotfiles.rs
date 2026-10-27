use std::env;
use std::ffi::OsString;
use std::io;
use std::process::ExitCode;

use forge::system::Machine;
use forge_apps::stow::{self, Environment};

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let user = env::var("USER").unwrap_or_default();
    ExitCode::from(stow::run(
        &Machine,
        &args,
        Environment {
            user: &user,
            out: io::stdout(),
            err: io::stderr(),
        },
    ))
}
