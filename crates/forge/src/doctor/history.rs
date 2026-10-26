use std::io::{self, Write};

use super::Context;
use crate::auto_upgrade::parse_history_line;

const ROWS: usize = 10;

pub(super) fn show(context: &Context, out: &mut dyn Write) -> io::Result<()> {
    let Some(history) = context.config.upgrade_history() else {
        return writeln!(out, "auto-upgrade is off on this host");
    };

    let text = context
        .system
        .files
        .read(&history)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    if text.is_empty() {
        return writeln!(out, "auto-upgrade never ran yet");
    }

    write!(out, "{}", table(&text))
}

fn table(history: &str) -> String {
    let rows: Vec<_> = history.lines().filter_map(parse_history_line).collect();
    let shown = &rows[rows.len().saturating_sub(ROWS)..];

    let mut lines = vec![row("TIME", "OUTCOME", "FROM", "TO", "SOURCE")];
    for run in shown {
        lines.push(row(
            &run.time,
            run.outcome.as_str(),
            &cell(&hash(&run.from)),
            &cell(&hash(&run.to)),
            &cell(&run.source),
        ));
    }
    lines.join("\n") + "\n"
}

fn row(time: &str, outcome: &str, from: &str, to: &str, source: &str) -> String {
    format!("{time:<25}  {outcome:<11}  {from:<12}  {to:<12}  {source}")
}

fn hash(value: &str) -> String {
    value.chars().take(12).collect()
}

fn cell(value: &str) -> String {
    if value.is_empty() {
        return "-".to_owned();
    }
    value.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_keeps_the_last_ten_runs_with_short_hashes() {
        let history = (1..=12)
            .map(|day| format!("2026-01-{day:02}T04:00:00+00:00\tupgraded\taaaaaaaaaaaaaaaaaaaa\t\thttps://example.org/fork.git"))
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!(table(&history));
    }
}
