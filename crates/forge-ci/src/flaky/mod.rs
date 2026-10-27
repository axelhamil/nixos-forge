use std::io::Write;

use forge::system::Commands;

use crate::badges_branch::{self, ReadError};
use crate::model::{FlakyHistory, json_or_none};

pub mod plan;
pub mod report;
pub mod run;

pub const RUNS: u32 = 5;

const FILE: &str = "flaky.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "flaky-history: without the record, every VM check would be measured again; fix the fetch instead"
)]
pub struct HistoryUnreadable;

pub(crate) fn history(
    commands: &dyn Commands,
    err: &mut dyn Write,
) -> Result<FlakyHistory, HistoryUnreadable> {
    let text = match badges_branch::read(commands, ".", FILE) {
        Ok(text) => Some(text),
        Err(error) => {
            let _ = writeln!(err, "ci-badges: {error}");
            if error == ReadError::Fetch {
                return Err(HistoryUnreadable);
            }
            None
        }
    };

    if let Some(history) = json_or_none(text.as_deref()) {
        return Ok(history);
    }

    let _ = writeln!(
        err,
        "flaky-history: no valid flaky.json on the badges branch, every VM check is measured"
    );
    Ok(FlakyHistory::default())
}
