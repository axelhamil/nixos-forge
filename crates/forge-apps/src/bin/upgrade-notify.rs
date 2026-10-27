use std::env;
use std::io;
use std::process::ExitCode;

use forge::system::Machine;
use forge_apps::upgrade_notify::{self, Paths};

fn main() -> ExitCode {
    let home = env::var("HOME").unwrap_or_default();
    let xdg_state_home = env::var("XDG_STATE_HOME").ok();
    let paths = Paths::of_session(xdg_state_home.as_deref(), &home);
    ExitCode::from(upgrade_notify::run(&Machine, &paths, &mut io::stderr()))
}
