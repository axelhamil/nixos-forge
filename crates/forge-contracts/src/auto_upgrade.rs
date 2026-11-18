use serde::{Deserialize, Deserializer, Serialize};

pub const STATE_FILE: &str = "state.json";
pub const HISTORY_FILE: &str = "history.tsv";

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

impl State {
    #[must_use]
    pub fn load(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }

    #[must_use]
    pub fn json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    #[must_use]
    pub fn history_row(&self) -> HistoryRow {
        HistoryRow {
            time: self.time.clone(),
            outcome: self.outcome,
            from: self.from.clone(),
            to: self.to.clone(),
            source: self.source.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub time: String,
    pub outcome: Outcome,
    pub from: String,
    pub to: String,
    pub source: String,
}

impl HistoryRow {
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let mut fields = line.split('\t');
        let time = fields.next()?.to_owned();
        let outcome = Outcome::parse(fields.next()?)?;
        let mut field = || fields.next().unwrap_or_default().to_owned();
        Some(Self {
            time,
            outcome,
            from: field(),
            to: field(),
            source: field(),
        })
    }

    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\n",
            self.time,
            self.outcome.as_str(),
            self.from,
            self.to,
            self.source
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        State {
            time: "2026-10-23T04:00:00+02:00".to_owned(),
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
    fn a_full_state_serializes_to_the_seven_fields() {
        let full = State {
            outcome: Outcome::Upgraded,
            changes: "nixpkgs: 24.11 -> 25.05".to_owned(),
            ..state()
        };

        insta::assert_snapshot!(serde_json::to_string_pretty(&full).unwrap());
        assert_eq!(State::load(full.json().as_bytes()), Some(full));
    }

    #[test]
    fn a_state_without_its_optional_fields_reads_them_as_empty() {
        let state =
            State::load(br#"{"time":"2026-10-23T04:00:00+02:00","outcome":"waiting","from":null}"#)
                .unwrap();

        assert_eq!(state.outcome, Outcome::Waiting);
        assert_eq!(state.from, "");
        assert_eq!(state.changes, "");
    }

    #[test]
    fn a_history_row_round_trips() {
        let row = state().history_row();

        assert_eq!(
            row.line(),
            format!(
                "2026-10-23T04:00:00+02:00\tup-to-date\t{}\t{}\thttps://github.com/alice/nixos.git\n",
                "a".repeat(40),
                "b".repeat(40)
            )
        );
        assert_eq!(HistoryRow::parse(row.line().trim_end()), Some(row));
    }

    #[test]
    fn a_history_row_with_an_unknown_outcome_is_skipped() {
        assert_eq!(
            HistoryRow::parse("2026-10-23T04:00:00+02:00\tbogus\ta\tb\tc"),
            None
        );
    }

    #[test]
    fn a_short_history_row_fills_the_missing_fields_empty() {
        let row = HistoryRow::parse("2026-10-23T04:00:00+02:00\trolled-back").unwrap();

        assert_eq!(row.outcome, Outcome::RolledBack);
        assert_eq!(
            (row.from.as_str(), row.to.as_str(), row.source.as_str()),
            ("", "", "")
        );
    }
}
