use std::io;

use jiff::civil::DateTime;
use jiff::tz::TimeZone;

use super::format::duration;
use super::report::{Report, Status};
use super::{Context, Invocation};

const CHECK: &str = "Snapshots";
const SERVICE: &str = "btrbk-snapshots.service";
const DEFAULT_INTERVAL: i64 = 3600;

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let Some(calendar) = context
        .config
        .btrbk_calendar
        .as_deref()
        .filter(|calendar| !calendar.is_empty())
    else {
        return report.line(Status::Skip, CHECK, "no btrbk snapshots on this host");
    };

    let result = context.system.commands.run(&Invocation::new(
        "systemctl",
        &["show", "-p", "Result", "--value", SERVICE],
    ));
    if result.stdout.trim_end() != "success" {
        return report.hinted(
            Status::Fail,
            CHECK,
            "the last btrbk run failed",
            "journalctl -u btrbk-snapshots",
        );
    }

    let interval = calendar_interval(context, calendar);
    let last = unix_property(context, "ExecMainStartTimestamp", SERVICE).max(unix_property(
        context,
        "LastTriggerUSec",
        "btrbk-snapshots.timer",
    ));

    if last == 0 {
        return report.hinted(
            Status::Warn,
            CHECK,
            "btrbk never ran",
            "systemctl start btrbk-snapshots, then systemctl list-timers btrbk-snapshots",
        );
    }

    let age = context.now.as_second() - last;
    let schedule = format!(
        "last btrbk run {} ago, scheduled every {}",
        duration(age),
        duration(interval)
    );
    if age > 2 * interval {
        return report.hinted(
            Status::Warn,
            CHECK,
            &schedule,
            "systemctl list-timers btrbk-snapshots",
        );
    }

    let dir = &context.config.snapshot_dir;
    let names = context.system.files.list(dir);
    let missing: Vec<&str> = context
        .config
        .snapshot_subvolumes
        .iter()
        .filter(|subvolume| {
            let prefix = format!("{subvolume}.");
            !names.iter().any(|name| name.starts_with(&prefix))
        })
        .map(String::as_str)
        .collect();

    if !missing.is_empty() {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("no snapshot of {} in {}", missing.join(" "), dir.display()),
            "journalctl -u btrbk-snapshots",
        );
    }
    report.line(Status::Ok, CHECK, &schedule)
}

fn calendar_interval(context: &Context, calendar: &str) -> i64 {
    let answer = context.system.commands.run(
        &Invocation::new("systemd-analyze", &["calendar", "--iterations=2", calendar])
            .env("TZ", "UTC"),
    );
    let elapses: Vec<&str> = answer
        .stdout
        .lines()
        .filter(|line| {
            line.contains("Next elapse")
                || line.contains("Iter. #2")
                || line.contains("Iteration #2")
        })
        .filter_map(|line| line.split(": ").nth(1))
        .collect();

    let [first, second, ..] = elapses.as_slice() else {
        return DEFAULT_INTERVAL;
    };
    match (utc_seconds(first), utc_seconds(second)) {
        (Some(first), Some(second)) => second - first,
        _ => DEFAULT_INTERVAL,
    }
}

fn utc_seconds(elapse: &str) -> Option<i64> {
    let mut words = elapse.split_whitespace().skip(1);
    let civil = format!("{} {}", words.next()?, words.next()?);
    let datetime: DateTime = civil.parse().ok()?;
    let zoned = datetime.to_zoned(TimeZone::UTC).ok()?;
    Some(zoned.timestamp().as_second())
}

fn unix_property(context: &Context, property: &str, unit: &str) -> i64 {
    let answer = context.system.commands.run(&Invocation::new(
        "systemctl",
        &["show", "--timestamp=unix", "-p", property, "--value", unit],
    ));
    let value = answer.stdout.trim_end();
    let value = value.strip_prefix('@').unwrap_or(value);
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return 0;
    }
    value.parse().unwrap_or(0)
}
