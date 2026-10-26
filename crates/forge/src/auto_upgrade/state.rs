use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use jiff::tz::TimeZone;
use rustix::fs::{FlockOperation, flock};
use rustix::io::Errno;
use serde::{Deserialize, Deserializer, Serialize};

pub const STATE_FILE: &str = "state.json";
pub const HISTORY_FILE: &str = "history.tsv";
const LOCK_FILE: &str = "lock";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Upgraded,
    UpToDate,
    Waiting,
    Unreachable,
    Refused,
    RolledBack,
    Failed,
}

impl Outcome {
    const ALL: [Self; 7] = [
        Self::Upgraded,
        Self::UpToDate,
        Self::Waiting,
        Self::Unreachable,
        Self::Refused,
        Self::RolledBack,
        Self::Failed,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upgraded => "upgraded",
            Self::UpToDate => "up-to-date",
            Self::Waiting => "waiting",
            Self::Unreachable => "unreachable",
            Self::Refused => "refused",
            Self::RolledBack => "rolled-back",
            Self::Failed => "failed",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|outcome| outcome.as_str() == text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub time: String,
    pub outcome: Outcome,
    #[serde(default, deserialize_with = "text")]
    pub from: String,
    #[serde(default, deserialize_with = "text")]
    pub to: String,
    #[serde(default, deserialize_with = "text")]
    pub source: String,
    #[serde(default, deserialize_with = "text")]
    pub message: String,
    #[serde(default, deserialize_with = "text")]
    pub changes: String,
}

fn text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub time: String,
    pub outcome: Outcome,
    pub from: String,
    pub to: String,
    pub source: String,
}

#[must_use]
pub fn parse_history_line(line: &str) -> Option<Row> {
    let mut fields = line.split('\t');
    let time = fields.next()?.to_owned();
    let outcome = Outcome::parse(fields.next()?)?;
    let mut field = || fields.next().unwrap_or_default().to_owned();
    Some(Row {
        time,
        outcome,
        from: field(),
        to: field(),
        source: field(),
    })
}

impl State {
    #[must_use]
    pub fn load(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }

    fn json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    fn history_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\n",
            self.time,
            self.outcome.as_str(),
            self.from,
            self.to,
            self.source
        )
    }

    pub(super) fn summary(&self) -> String {
        format!("{}: {}\n", self.outcome.as_str(), self.message)
    }

    pub(super) fn record(&self, dir: &Path) -> io::Result<()> {
        let fresh = dir.join(format!("{STATE_FILE}.new"));
        fs::write(&fresh, self.json())?;
        fs::rename(&fresh, dir.join(STATE_FILE))?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(HISTORY_FILE))?
            .write_all(self.history_line().as_bytes())
    }
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
    fn the_state_file_keeps_the_seven_fields_of_the_shell_version() {
        insta::assert_snapshot!(state().json());
    }

    #[test]
    fn a_run_appends_one_history_row_and_replaces_the_state() {
        let dir = tempfile::tempdir().unwrap();

        state().record(dir.path()).unwrap();
        state().record(dir.path()).unwrap();

        insta::assert_snapshot!(fs::read_to_string(dir.path().join("history.tsv")).unwrap());
        assert!(!dir.path().join("state.json.new").exists());
        assert_eq!(
            state().summary(),
            "up-to-date: deploy/machine is the running commit\n"
        );
    }
}
