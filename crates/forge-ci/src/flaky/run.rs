use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use forge::system::{Commands, Invocation, Machine, Streams};
use serde_json::Number;

use crate::model::{FlakyRecord, FlakyRun};

const USAGE: &str = "usage: ci-flaky <system> <check> <runs> <out-dir>";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    system: String,
    check: String,
    runs: u32,
    out: PathBuf,
}

enum Parsed {
    Help,
    Run(Request),
}

#[derive(Debug, thiserror::Error)]
enum RunError {
    #[error("cannot {action}: {error}")]
    Io {
        action: &'static str,
        error: io::Error,
    },
    #[error("cannot run nix build: {0}")]
    Stream(io::Error),
}

fn io_error(action: &'static str) -> impl FnOnce(io::Error) -> RunError {
    move |error| RunError::Io { action, error }
}

fn parse(args: &[OsString]) -> Option<Parsed> {
    if matches!(
        args.first().and_then(|arg| arg.to_str()),
        Some("-h" | "--help")
    ) {
        return Some(Parsed::Help);
    }

    let [system, check, runs, out] = args else {
        return None;
    };
    let runs = runs.to_str()?;
    if !runs.starts_with(|first: char| ('1'..='9').contains(&first))
        || !runs.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    Some(Parsed::Run(Request {
        system: system.to_str()?.to_owned(),
        check: check.to_str()?.to_owned(),
        runs: runs.parse().ok()?,
        out: PathBuf::from(out),
    }))
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    match parse(args) {
        Some(Parsed::Help) => {
            println!("{USAGE}");
            0
        }
        Some(Parsed::Run(request)) => match measure(&Machine, &Machine, &request) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("ci-flaky: {error}");
                1
            }
        },
        None => {
            eprintln!("{USAGE}");
            2
        }
    }
}

fn only_differs(log: &str) -> bool {
    log.contains("may not be deterministic")
        && !log.contains("error: builder for")
        && !log.contains("error: Cannot build")
}

fn build(installable: &str, rebuild: bool) -> Invocation {
    let mut args = vec!["build", "--no-link", "--print-build-logs"];
    if rebuild {
        args.push("--rebuild");
    }
    args.push(installable);
    Invocation::new("nix", &args)
}

fn measure(
    commands: &dyn Commands,
    streams: &dyn Streams,
    request: &Request,
) -> Result<(), RunError> {
    let Request {
        system,
        check,
        runs,
        out,
    } = request;
    let installable = format!(".#checks.{system}.{check}");
    let work = tempfile::tempdir().map_err(io_error("create a temporary directory"))?;
    let log = work.path().join("log");
    fs::create_dir_all(out).map_err(io_error("create the results directory"))?;

    println!("ci-flaky: realising {check} once, not counted as a run");
    let mut realised = commands.run(&build(&installable, false).attached()).success;
    if !realised {
        eprintln!("ci-flaky: {check} did not realise, each run builds it until one passes");
    }

    let mut record = FlakyRecord {
        check: Some(check.clone()),
        system: Some(system.clone()),
        runs: Vec::new(),
    };
    for run in 1..=*runs {
        let start = Instant::now();
        let streamed = streams
            .stream(&build(&installable, realised), &log)
            .map_err(RunError::Stream)?;
        let passed = streamed.succeeded || (realised && logged_only_differs(&log));
        realised |= passed;

        record.runs.push(FlakyRun {
            passed,
            seconds: Number::from(start.elapsed().as_secs()),
        });
        println!("ci-flaky: run {run} of {runs} of {check} passed: {passed}");
    }

    write_record(&out.join(format!("{system}-{check}.json")), &record)
}

fn logged_only_differs(log: &Path) -> bool {
    fs::read(log).is_ok_and(|bytes| only_differs(&String::from_utf8_lossy(&bytes)))
}

fn write_record(path: &Path, record: &FlakyRecord) -> Result<(), RunError> {
    let mut text = serde_json::to_string_pretty(record)
        .map_err(|error| io_error("write the result")(error.into()))?;
    text.push('\n');
    fs::write(path, text).map_err(io_error("write the result"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Option<Request> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        match parse(&args)? {
            Parsed::Run(request) => Some(request),
            Parsed::Help => None,
        }
    }

    #[test]
    fn the_run_count_is_a_positive_number_without_a_leading_zero() {
        assert_eq!(parsed(&["x", "c", "12", "out"]).unwrap().runs, 12);
        for runs in ["0", "05", "-1", "+3", "", "2x"] {
            assert_eq!(parsed(&["x", "c", runs, "out"]), None, "{runs}");
        }
        assert_eq!(parsed(&["x", "c", "5"]), None);
    }

    #[test]
    fn a_log_that_only_differs_counts_as_a_pass() {
        assert!(only_differs(
            "error: derivation '/nix/store/x.drv' may not be deterministic: output differs"
        ));
        assert!(!only_differs(
            "may not be deterministic\nerror: builder for '/nix/store/x.drv' failed"
        ));
        assert!(!only_differs(
            "may not be deterministic\nerror: Cannot build '/nix/store/x.drv'"
        ));
        assert!(!only_differs(
            "error: builder for '/nix/store/x.drv' failed"
        ));
    }
}
