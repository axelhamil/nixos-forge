use std::collections::HashMap;

use jiff::civil::date;
use jiff::tz::TimeZone;

use super::{Context, Invocation};

const MAX_CLOCK_STEP: i64 = 2_000_000;
const PROPERTIES: [&str; 2] = ["InactiveExitTimestamp", "ActiveEnterTimestamp"];

pub(super) fn end(context: &Context) -> i64 {
    let answer = context.system.commands.run(&Invocation::new(
        "systemctl",
        &[
            "show",
            "--timestamp=us+utc",
            "-p",
            "Id,InactiveExitTimestamp,InactiveExitTimestampMonotonic,ActiveEnterTimestamp,ActiveEnterTimestampMonotonic",
            "systemd-tmpfiles-setup.service",
            "sysinit.target",
        ],
    ));
    end_from(&answer.stdout)
}

fn end_from(show: &str) -> i64 {
    let mut latest: Option<i64> = None;
    let mut reached = 0;
    let mut offset_at_reached = 0;

    for block in show.split("\n\n") {
        let stamp: HashMap<&str, &str> = block
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect();

        for property in PROPERTIES {
            if let Some(offset) = clock_offset(&stamp, property) {
                latest = Some(latest.map_or(offset, |latest| latest.max(offset)));
            }
        }

        if stamp.get("Id") == Some(&"sysinit.target") {
            reached = number(&stamp, "ActiveEnterTimestampMonotonic");
            offset_at_reached = clock_offset(&stamp, "ActiveEnterTimestamp").unwrap_or(0);
        }
    }

    let end = reached + latest.unwrap_or(0);
    end.min(reached + offset_at_reached + MAX_CLOCK_STEP)
}

fn number(stamp: &HashMap<&str, &str>, property: &str) -> i64 {
    stamp
        .get(property)
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
}

fn clock_offset(stamp: &HashMap<&str, &str>, property: &str) -> Option<i64> {
    let monotonic = number(stamp, &format!("{property}Monotonic"));
    if monotonic == 0 {
        return None;
    }

    let parts: Vec<&str> = stamp.get(property)?.split([' ', ':', '.', '-']).collect();
    let [_, year, month, day, hour, minute, second, micros, _, ..] = parts.as_slice() else {
        return None;
    };
    let wall = date(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
        .at(
            hour.parse().ok()?,
            minute.parse().ok()?,
            second.parse().ok()?,
            0,
        )
        .to_zoned(TimeZone::UTC)
        .ok()?
        .timestamp()
        .as_second();
    let micros: i64 = micros.parse().ok()?;
    Some(wall * 1_000_000 + micros - monotonic)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(
        tmpfiles_exit: &str,
        tmpfiles_enter: &str,
        sysinit: &str,
        sysinit_monotonic: u64,
    ) -> String {
        format!(
            "Id=systemd-tmpfiles-setup.service\n\
             InactiveExitTimestamp=Fri 2026-10-23 {tmpfiles_exit} UTC\n\
             InactiveExitTimestampMonotonic=9481224\n\
             ActiveEnterTimestamp=Fri 2026-10-23 {tmpfiles_enter} UTC\n\
             ActiveEnterTimestampMonotonic=9519561\n\
             \n\
             Id=sysinit.target\n\
             InactiveExitTimestamp=Fri 2026-10-23 {sysinit} UTC\n\
             InactiveExitTimestampMonotonic={sysinit_monotonic}\n\
             ActiveEnterTimestamp=Fri 2026-10-23 {sysinit} UTC\n\
             ActiveEnterTimestampMonotonic={sysinit_monotonic}\n"
        )
    }

    fn micros(time: &str) -> i64 {
        let timestamp: jiff::Timestamp = format!("2026-10-23T{time}Z").parse().unwrap();
        timestamp.as_microsecond()
    }

    #[test]
    fn a_steady_clock_ends_the_setup_when_sysinit_is_reached() {
        let output = show(
            "07:35:19.717159",
            "07:35:19.755497",
            "07:35:19.767315",
            9_531_379,
        );

        assert_eq!(end_from(&output), micros("07:35:19.767315"));
    }

    #[test]
    fn a_clock_stepped_back_keeps_the_offset_tmpfiles_wrote_with() {
        let output = show(
            "07:35:19.717159",
            "07:35:19.755497",
            "07:35:18.267315",
            9_531_379,
        );

        assert_eq!(end_from(&output), micros("07:35:19.767315"));
    }

    #[test]
    fn a_step_beyond_two_seconds_is_capped() {
        let output = show(
            "07:35:29.717159",
            "07:35:29.755497",
            "07:35:19.767315",
            9_531_379,
        );

        assert_eq!(end_from(&output), micros("07:35:21.767315"));
    }

    #[test]
    fn missing_timestamps_count_as_zero() {
        assert_eq!(end_from(""), 0);
        assert_eq!(
            end_from(
                "Id=sysinit.target\nActiveEnterTimestamp=n/a\nActiveEnterTimestampMonotonic=0\n"
            ),
            0
        );
    }
}
