use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use forge::auto_upgrade::{Outcome, STATE_FILE, State};
use forge::system::{Commands, Invocation};

pub const STATE_DIRECTORY: &str = "/var/lib/auto-upgrade";
pub const NIXOS_VERSION: &str = "/run/current-system/sw/bin/nixos-version";

const RESTART: &str = "\nRestart to use it.";

#[derive(Debug, thiserror::Error)]
#[error("cannot write {path}: {source}")]
struct SeenError {
    path: PathBuf,
    source: io::Error,
}

pub struct Paths {
    pub state: PathBuf,
    pub nixos_version: String,
    pub seen_directory: PathBuf,
}

impl Paths {
    #[must_use]
    pub fn of_session(xdg_state_home: Option<&str>, home: &str) -> Self {
        let state_home = xdg_state_home
            .filter(|path| !path.is_empty())
            .map_or_else(|| format!("{home}/.local/state"), str::to_owned);
        Self {
            state: Path::new(STATE_DIRECTORY).join(STATE_FILE),
            nixos_version: NIXOS_VERSION.to_owned(),
            seen_directory: PathBuf::from(state_home).join("upgrade-notify"),
        }
    }
}

pub fn run(commands: &impl Commands, paths: &Paths, err: &mut impl Write) -> u8 {
    let Some(state) = fs::read(&paths.state)
        .ok()
        .and_then(|bytes| State::load(&bytes))
    else {
        return 0;
    };
    if state.outcome != Outcome::Upgraded {
        return 0;
    }

    let upgrade = format!("{} {}", state.time, state.to);
    let seen = paths.seen_directory.join("seen");
    if fs::read_to_string(&seen).is_ok_and(|text| text.trim_end_matches('\n') == upgrade) {
        return 0;
    }

    let running = commands.run(&Invocation::new(
        &paths.nixos_version,
        &["--configuration-revision"],
    ));
    let body = body(&state, running.line());

    let notified = commands.run(
        &Invocation::new(
            "notify-send",
            &["--app-name=auto-upgrade", "System updated", &body],
        )
        .loud(),
    );
    if !notified.success {
        let _ = writeln!(
            err,
            "no notification daemon answered; the upgrade is announced at the next session"
        );
        return 0;
    }

    if let Err(error) = remember(paths, &upgrade) {
        let _ = writeln!(err, "upgrade-notify: {error}");
        return 1;
    }
    0
}

fn body(state: &State, running: &str) -> String {
    let text = if state.changes.is_empty() {
        &state.message
    } else {
        &state.changes
    };
    let text = text.trim_end_matches('\n');
    if running == state.to {
        return text.to_owned();
    }
    format!("{text}{RESTART}")
}

fn remember(paths: &Paths, upgrade: &str) -> Result<(), SeenError> {
    let seen = paths.seen_directory.join("seen");
    let failed = |source| SeenError {
        path: seen.clone(),
        source,
    };
    fs::create_dir_all(&paths.seen_directory).map_err(failed)?;
    fs::write(&seen, format!("{upgrade}\n")).map_err(failed)
}
