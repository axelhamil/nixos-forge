use std::io;

use jiff::Timestamp;
use serde_json::Value;

use super::Context;
use super::format::duration;
use super::report::{Report, Status};

const CHECK: &str = "Upgrades";
const STALE_AFTER: i64 = 14 * 86400;

struct State {
    time: Timestamp,
    outcome: String,
    message: String,
    changes: String,
}

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let Some(state_file) = context.config.upgrade_state() else {
        return report.line(Status::Skip, CHECK, "auto-upgrade is off");
    };

    let Ok(bytes) = context.system.files.read(&state_file) else {
        return report.line(Status::Skip, CHECK, "never ran yet");
    };

    let Some(state) = parse_state(&bytes) else {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("cannot read {}", state_file.display()),
            "auto-upgrade writes it again at its next run: systemctl start auto-upgrade",
        );
    };

    let when = context.short_date(state.time);
    if !matches!(state.outcome.as_str(), "upgraded" | "up-to-date") {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("{} on {when}", state.outcome),
            &state.message,
        );
    }

    let age = context.now.as_second() - state.time.as_second();
    if age > STALE_AFTER {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("last run on {when}, {} ago", duration(age)),
            "systemctl list-timers auto-upgrade, journalctl -u auto-upgrade",
        );
    }

    if state.outcome == "upgraded" {
        let changes = if state.changes.is_empty() {
            &state.message
        } else {
            &state.changes
        };
        return report.line(Status::Ok, CHECK, &format!("upgraded {when}: {changes}"));
    }

    let since = up_to_date_since(context)
        .and_then(|since| since.parse::<Timestamp>().ok())
        .map_or(when, |since| context.short_date(since));
    report.line(Status::Ok, CHECK, &format!("up to date since {since}"))
}

fn parse_state(bytes: &[u8]) -> Option<State> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let time = value.get("time")?.as_str()?.parse().ok()?;
    let outcome = value.get("outcome")?.as_str()?.to_owned();
    Some(State {
        time,
        outcome,
        message: optional_text(value.get("message")),
        changes: optional_text(value.get("changes")),
    })
}

fn optional_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null | Value::Bool(false)) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

fn up_to_date_since(context: &Context) -> Option<String> {
    let bytes = context
        .system
        .files
        .read(&context.config.upgrade_history()?)
        .ok()?;
    let text = String::from_utf8_lossy(&bytes);

    let mut since: Option<&str> = None;
    for row in text.lines() {
        let mut fields = row.split('\t');
        let time = fields.next().unwrap_or_default();
        match fields.next().unwrap_or_default() {
            "upgraded" => since = Some(time),
            "up-to-date" if since.is_none_or(str::is_empty) => since = Some(time),
            "rolled-back" => since = None,
            _ => {}
        }
    }
    since.filter(|since| !since.is_empty()).map(str::to_owned)
}
