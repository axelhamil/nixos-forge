use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use forge::system::{Commands, Machine};
use serde::Deserialize;
use serde_json::{Number, Value};

use super::plan::{FlakyJob, tree};
use crate::badges::{FlakyHistoryError, flaky_history};
use crate::model::{FlakyHistory, FlakyRecord, FlakyRun, key};
use crate::results;

const USAGE: &str =
    "usage: MATRIX=<all> PLANNED=<checks> ci-flaky-report <results-dir> <history-out-dir>";

const INTRO: &str = "Each VM check is realised once, then rebuilt with `nix build --rebuild` for every run. A run that fails is a test that does not always pass on the same code. A check whose derivation already passed all of its runs is not rerun.";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Named {
    check: String,
    system: String,
}

#[derive(Debug, thiserror::Error)]
enum ReportError {
    #[error("ci-flaky-report: {variable} is not a JSON list of checks: {reason}")]
    Variable {
        variable: &'static str,
        reason: String,
    },
    #[error("ci-flaky-report: {} is not valid JSON: {reason}", .path.display())]
    Result { path: PathBuf, reason: String },
    #[error(transparent)]
    History(#[from] FlakyHistoryError),
    #[error("ci-flaky-report: cannot write {}: {error}", .path.display())]
    Write { path: PathBuf, error: io::Error },
}

struct Measure {
    total: usize,
    passed: usize,
    rerun: bool,
    median: String,
}

impl Measure {
    fn of(record: &FlakyRecord, rerun: bool) -> Self {
        let total = record.runs.len();
        let passed = record.runs.iter().filter(|run| run.passed).count();
        Self {
            total,
            passed,
            rerun,
            median: median(&record.runs),
        }
    }

    fn rate(&self) -> usize {
        self.passed * 100 / self.total.max(1)
    }
}

struct Row<'a> {
    job: &'a FlakyJob,
    record: Option<(&'a FlakyRecord, Measure)>,
}

fn median(runs: &[FlakyRun]) -> String {
    let mut seconds: Vec<&Number> = runs.iter().map(|run| &run.seconds).collect();
    seconds.sort_by(|a, b| {
        let value = |number: &Number| number.as_f64().unwrap_or(0.0);
        value(a).total_cmp(&value(b))
    });
    let middle = seconds.len() / 2;
    if seconds.len() % 2 == 1 {
        return seconds[middle].to_string();
    }

    let (Some(low), Some(high)) = (seconds.get(middle.wrapping_sub(1)), seconds.get(middle)) else {
        return "null".to_owned();
    };
    let sum = low.as_f64().unwrap_or(0.0) + high.as_f64().unwrap_or(0.0);
    let floored = (sum / 2.0).floor();
    format!("{floored}")
}

fn record_of(result: &Value) -> (String, FlakyRecord) {
    let text = |field: &str| result.get(field).and_then(Value::as_str).map(str::to_owned);
    let runs = result
        .get("runs")
        .and_then(|runs| serde_json::from_value::<Vec<FlakyRun>>(runs.clone()).ok())
        .unwrap_or_default();
    let check = text("check");
    let system = text("system");
    let key = key(
        system.as_deref().unwrap_or(""),
        check.as_deref().unwrap_or(""),
    );
    (
        key,
        FlakyRecord {
            check,
            system,
            runs,
        },
    )
}

pub struct Report {
    pub ok: bool,
    pub history: FlakyHistory,
    pub markdown: Vec<String>,
}

fn report(
    matrix: &[FlakyJob],
    planned: &[Named],
    results: &[Value],
    old: &FlakyHistory,
    tree: &str,
) -> Report {
    let by_check: HashMap<String, FlakyRecord> = results.iter().map(record_of).collect();
    let planned: HashSet<String> = planned
        .iter()
        .map(|job| key(&job.system, &job.check))
        .collect();

    let mut rows: Vec<Row> = matrix
        .iter()
        .map(|job| {
            let job_key = key(&job.system, &job.check);
            let recorded = job.drv_path.as_ref().and_then(|drv| old.checks.get(drv));
            let record = match (by_check.get(&job_key), recorded) {
                (Some(new), _) if !new.runs.is_empty() => Some((new, Measure::of(new, true))),
                (_, Some(recorded)) if !planned.contains(&job_key) => {
                    Some((recorded, Measure::of(recorded, false)))
                }
                _ => None,
            };
            Row { job, record }
        })
        .collect();
    rows.sort_by(|a, b| (&a.job.check, &a.job.system).cmp(&(&b.job.check, &b.job.system)));

    let flaky: Vec<&Row> = rows
        .iter()
        .filter(|row| {
            row.record
                .as_ref()
                .is_some_and(|(_, measure)| measure.passed < measure.total)
        })
        .collect();
    let missing: Vec<&Row> = rows.iter().filter(|row| row.record.is_none()).collect();
    let ok = flaky.is_empty() && missing.is_empty();

    let checks: BTreeMap<String, FlakyRecord> = rows
        .iter()
        .filter_map(|row| {
            let (record, _) = row.record.as_ref()?;
            Some((row.job.drv_path.clone()?, (*record).clone()))
        })
        .collect();

    let mut markdown: Vec<String> = [
        "## Flaky VM checks",
        "",
        INTRO,
        "",
        "| Check | System | Pass rate | Median time |",
        "|---|---|---|---|",
    ]
    .map(str::to_owned)
    .to_vec();
    markdown.extend(rows.iter().map(table_row));
    markdown.push(String::new());
    if ok {
        markdown.push("Every VM check passed all of its runs.".to_owned());
    }
    markdown.extend(flaky.iter().filter_map(|row| {
        let (_, measure) = row.record.as_ref()?;
        let runs = if measure.total == 1 { "run" } else { "runs" };
        Some(format!(
            "`{}` passed {} of its {} {runs}.",
            row.job.check, measure.passed, measure.total
        ))
    }));
    markdown.extend(missing.iter().map(|row| {
        format!(
            "`{}` wrote no result: its job failed or hit its time limit, see its log.",
            row.job.check
        )
    }));

    Report {
        ok,
        history: FlakyHistory {
            tree: ok.then(|| tree.to_owned()),
            checks,
        },
        markdown,
    }
}

fn table_row(row: &Row) -> String {
    let FlakyJob { check, system, .. } = row.job;
    let mut line = format!("| `{check}` | {system} | ");
    let _ = match &row.record {
        None => write!(line, "no result | |"),
        Some((_, measure)) if measure.rerun => {
            write!(line, "{} % | {} s |", measure.rate(), measure.median)
        }
        Some((_, measure)) => write!(
            line,
            "{} % (not rerun, unchanged) | {} s |",
            measure.rate(),
            measure.median
        ),
    };
    line
}

fn variable<T: for<'de> Deserialize<'de>>(
    variable: &'static str,
    text: &str,
) -> Result<T, ReportError> {
    serde_json::from_str(text).map_err(|error| ReportError::Variable {
        variable,
        reason: error.to_string(),
    })
}

fn load_results(dir: &Path) -> Result<Vec<Value>, ReportError> {
    let loaded = results::load::<Value>(dir);
    if let Some(unreadable) = loaded.unreadable.into_iter().next() {
        return Err(ReportError::Result {
            path: unreadable.path,
            reason: unreadable.reason,
        });
    }
    Ok(loaded.values)
}

fn write_history(dir: &Path, history: &FlakyHistory) -> Result<(), ReportError> {
    let path = dir.join("flaky.json");
    let failed = |error| ReportError::Write {
        path: path.clone(),
        error,
    };
    fs::create_dir_all(dir).map_err(failed)?;
    let mut text = serde_json::to_string_pretty(history).map_err(|error| failed(error.into()))?;
    text.push('\n');
    fs::write(&path, text).map_err(failed)
}

fn write_report(
    commands: &dyn Commands,
    matrix: &str,
    planned: &str,
    results_dir: &Path,
    history_dir: &Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<bool, ReportError> {
    let matrix: Vec<FlakyJob> = variable("MATRIX", matrix)?;
    let planned: Vec<Named> = variable("PLANNED", planned)?;
    let results = load_results(results_dir)?;
    let old = flaky_history(commands, ".", err)?;
    let tree = tree(commands).unwrap_or_default();

    let report = report(&matrix, &planned, &results, &old, &tree);
    write_history(history_dir, &report.history)?;
    for line in &report.markdown {
        let _ = writeln!(out, "{line}");
    }
    Ok(report.ok)
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    if args
        .first()
        .is_some_and(|arg| arg == "-h" || arg == "--help")
    {
        println!("{USAGE}");
        return 0;
    }

    let matrix = env::var("MATRIX").unwrap_or_default();
    let [results_dir, history_dir] = args else {
        eprintln!("{USAGE}");
        return 2;
    };
    if matrix.is_empty() {
        eprintln!("{USAGE}");
        return 2;
    }
    let planned = env::var("PLANNED")
        .ok()
        .filter(|planned| !planned.is_empty())
        .unwrap_or_else(|| "[]".to_owned());

    let written = write_report(
        &Machine,
        &matrix,
        &planned,
        Path::new(results_dir),
        Path::new(history_dir),
        &mut io::stdout(),
        &mut io::stderr(),
    );
    match written {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(seconds: &[Value]) -> Vec<FlakyRun> {
        seconds
            .iter()
            .map(|seconds| FlakyRun {
                passed: true,
                seconds: serde_json::from_value(seconds.clone()).unwrap(),
            })
            .collect()
    }

    #[test]
    fn the_median_keeps_an_odd_middle_and_floors_an_even_mean() {
        assert_eq!(median(&runs(&[5.into(), 1.into(), 4.into()])), "4");
        assert_eq!(median(&runs(&[7.5.into(), 1.into(), 9.into()])), "7.5");
        assert_eq!(median(&runs(&[1.into(), 2.into()])), "1");
        assert_eq!(median(&runs(&[2.5.into(), 3.into()])), "2");
    }
}
