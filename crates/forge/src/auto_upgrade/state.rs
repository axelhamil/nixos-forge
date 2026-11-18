use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use forge_contracts::auto_upgrade::{HISTORY_FILE, STATE_FILE, State};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rustix::fs::{FlockOperation, flock};
use rustix::io::Errno;

const LOCK_FILE: &str = "lock";

pub(super) fn summary(state: &State) -> String {
    format!("{}: {}\n", state.outcome.as_str(), state.message)
}

pub(super) fn record(state: &State, dir: &Path) -> io::Result<()> {
    let fresh = dir.join(format!("{STATE_FILE}.new"));
    fs::write(&fresh, state.json())?;
    fs::rename(&fresh, dir.join(STATE_FILE))?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(HISTORY_FILE))?
        .write_all(state.history_row().line().as_bytes())
}

pub(super) fn iso_time(now: Timestamp, zone: &TimeZone) -> String {
    now.to_zoned(zone.clone())
        .strftime("%Y-%m-%dT%H:%M:%S%:z")
        .to_string()
}

pub(super) fn lock_path(dir: &Path) -> PathBuf {
    dir.join(LOCK_FILE)
}

pub(super) fn lock(dir: &Path) -> io::Result<Option<File>> {
    fs::create_dir_all(dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock_path(dir))?;
    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(Some(file)),
        Err(Errno::WOULDBLOCK) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use forge_contracts::auto_upgrade::Outcome;

    use super::*;

    fn state() -> State {
        State {
            time: iso_time(
                "2026-10-23T02:00:00Z".parse().unwrap(),
                &TimeZone::fixed(jiff::tz::offset(2)),
            ),
            outcome: Outcome::UpToDate,
            from: "a".repeat(40),
            to: "b".repeat(40),
            source: "https://github.com/alice/nixos.git".to_owned(),
            message: "deploy/machine is the running commit".to_owned(),
            changes: String::new(),
        }
    }

    #[test]
    fn a_run_appends_one_history_row_and_replaces_the_state() {
        let dir = tempfile::tempdir().unwrap();

        record(&state(), dir.path()).unwrap();
        record(&state(), dir.path()).unwrap();

        insta::assert_snapshot!(fs::read_to_string(dir.path().join("history.tsv")).unwrap());
        assert!(!dir.path().join("state.json.new").exists());
        assert_eq!(
            summary(&state()),
            "up-to-date: deploy/machine is the running commit\n"
        );
    }
}
