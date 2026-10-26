use std::io;

use jiff::Timestamp;

use super::Context;
use super::format::duration;
use super::report::{Report, Status};
use crate::auto_upgrade::{Outcome, State, parse_history_line};

const CHECK: &str = "Upgrades";
const STALE_AFTER: i64 = 14 * 86400;

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let Some(state_file) = context.config.upgrade_state() else {
        return report.line(Status::Skip, CHECK, "auto-upgrade is off");
    };

    let Ok(bytes) = context.system.files.read(&state_file) else {
        return report.line(Status::Skip, CHECK, "never ran yet");
    };

    let Some((state, time)) = State::load(&bytes).and_then(|state| {
        let time = state.time.parse::<Timestamp>().ok()?;
        Some((state, time))
    }) else {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("cannot read {}", state_file.display()),
            "auto-upgrade writes it again at its next run: systemctl start auto-upgrade",
        );
    };

    let when = context.short_date(time);
    if !matches!(state.outcome, Outcome::Upgraded | Outcome::UpToDate) {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("{} on {when}", state.outcome.as_str()),
            &state.message,
        );
    }

    let age = context.now.as_second() - time.as_second();
    if age > STALE_AFTER {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("last run on {when}, {} ago", duration(age)),
            "systemctl list-timers auto-upgrade, journalctl -u auto-upgrade",
        );
    }

    if state.outcome == Outcome::Upgraded {
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

fn up_to_date_since(context: &Context) -> Option<String> {
    let bytes = context
        .system
        .files
        .read(&context.config.upgrade_history()?)
        .ok()?;
    let text = String::from_utf8_lossy(&bytes);

    let mut since: Option<String> = None;
    for row in text.lines().filter_map(parse_history_line) {
        match row.outcome {
            Outcome::Upgraded => since = Some(row.time),
            Outcome::UpToDate if since.as_deref().is_none_or(str::is_empty) => {
                since = Some(row.time);
            }
            Outcome::RolledBack => since = None,
            _ => {}
        }
    }
    since.filter(|since| !since.is_empty())
}
