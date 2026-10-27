use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use forge::system::{Commands, Machine};

use crate::badges_branch::{self, ReadError};
use crate::model::{
    Badge, BootTime, CheckResult, FlakyHistory, HostChecks, PlannedJob, SkippedJob, Status,
    Verdict, env_json, json_or_none, seconds, statuses,
};
use crate::results;

const USAGE: &str = "usage: ci-badges checks <out-dir>
       ci-badges reproducibility <summary.md> <out-dir>
       ci-badges publish <dir>
       ci-badges read <file>";

const USAGE_ERROR: u8 = 2;
const FAILED: u8 = 1;
const BADGE_COLOR: &str = "5277C3";
const FLAKY_FILE: &str = "flaky.json";

#[derive(Debug, thiserror::Error)]
pub enum FlakyHistoryError {
    #[error(
        "flaky-history: without the record, every VM check would be measured again; fix the fetch instead"
    )]
    Fetch,
}

#[derive(Debug, thiserror::Error)]
enum BadgesError {
    #[error("{0} does not exist")]
    NoSummary(String),
    #[error("cannot {action}: {error}")]
    Io { action: String, error: io::Error },
}

fn io_error(action: &str) -> impl FnOnce(io::Error) -> BadgesError {
    let action = action.to_owned();
    move |error| BadgesError::Io { action, error }
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let args: Vec<String> = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut out = io::stdout();
    let mut err = io::stderr();

    let done = match args.as_slice() {
        ["checks", out_dir] => checks(Path::new(out_dir), &mut out),
        ["reproducibility", summary, out_dir] => {
            reproducibility(Path::new(summary), Path::new(out_dir), &mut out)
        }
        ["publish", dir] => return publish(Path::new(dir), &mut out, &mut err),
        ["read", file] => return read(file, &mut out, &mut err),
        ["-h" | "--help", ..] => {
            let _ = writeln!(out, "{USAGE}");
            return 0;
        }
        _ => {
            let _ = writeln!(err, "{USAGE}");
            return USAGE_ERROR;
        }
    };

    match done {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(err, "ci-badges: {error}");
            FAILED
        }
    }
}

fn write_badge(path: &Path, badge: &Badge) -> Result<String, BadgesError> {
    let text = serde_json::to_string_pretty(badge).map_err(|error| BadgesError::Io {
        action: "serialize a badge".to_owned(),
        error: error.into(),
    })? + "\n";
    fs::write(path, &text).map_err(io_error(&format!("write {}", path.display())))?;
    Ok(text)
}

struct Run {
    planned: Option<Vec<PlannedJob>>,
    skipped: Vec<SkippedJob>,
    hosts: HostChecks,
    results: Vec<CheckResult>,
    current_boot: BTreeMap<String, BootTime>,
    lint_passed: bool,
    host: String,
}

impl Run {
    fn from_env() -> Self {
        let results_dir = env::var("RESULTS_DIR")
            .ok()
            .filter(|dir| !dir.is_empty())
            .unwrap_or_else(|| "results".to_owned());
        let results_dir = Path::new(&results_dir);
        Self {
            planned: env_json("PLANNED"),
            skipped: env_json("SKIPPED").unwrap_or_default(),
            hosts: env_json("HOST_CHECKS").unwrap_or_default(),
            results: results::check_results(results_dir).values,
            current_boot: results::boot_times(&results_dir.join("current/boot-time.json")),
            lint_passed: env::var("LINT_RESULT").is_ok_and(|lint| lint == "success"),
            host: env::var("BADGE_HOST")
                .ok()
                .filter(|host| !host.is_empty())
                .unwrap_or_else(|| "example".to_owned()),
        }
    }

    fn badges(&self) -> Vec<(&'static str, Badge)> {
        let Some(planned) = &self.planned else {
            return vec![(
                "checks.json",
                Badge::new("checks", "flake does not evaluate", "red"),
            )];
        };

        let jobs = statuses(planned, &self.skipped, &self.results);
        let ok = |status: &Status| *status == Status::Passed;
        let total = jobs.len() + 1;
        let passing =
            jobs.iter().filter(|job| ok(&job.status)).count() + usize::from(self.lint_passed);
        let checks = if passing == total {
            Badge::new("checks", &format!("{total}/{total} passing"), "brightgreen")
        } else {
            Badge::new(
                "checks",
                &format!("{}/{total} failing", total - passing),
                "red",
            )
        };

        let vm_tests = jobs.iter().filter(|job| job.vm).count();

        let builds = |name: &str, check: &str| {
            check == format!("{name}-toplevel") || check == format!("{name}-sd-image")
        };
        let machines: Vec<bool> = self
            .hosts
            .iter()
            .filter(|(name, checks)| checks.iter().any(|check| builds(name, check)))
            .map(|(name, _)| {
                jobs.iter()
                    .filter(|job| builds(name, job.check))
                    .all(|job| ok(&job.status))
            })
            .collect();
        let built = machines.iter().filter(|built| **built).count();
        let hosts = if built == machines.len() {
            Badge::new("hosts", &format!("{built} built"), BADGE_COLOR)
        } else {
            Badge::new("hosts", &format!("{built}/{} built", machines.len()), "red")
        };

        let mut badges = vec![
            ("checks.json", checks),
            (
                "vm-tests.json",
                Badge::new("vm tests", &vm_tests.to_string(), BADGE_COLOR),
            ),
            ("hosts.json", hosts),
        ];
        if let Some(boot) = self.boot_time() {
            badges.push((
                "boot-time.json",
                Badge::new("boot to multi-user", &seconds(boot), BADGE_COLOR),
            ));
        }
        badges
    }

    fn boot_time(&self) -> Option<f64> {
        let booting: Vec<&String> = self
            .hosts
            .iter()
            .filter(|(name, checks)| checks.contains(&format!("{name}-boots")))
            .map(|(name, _)| name)
            .collect();
        let host = booting
            .iter()
            .find(|name| **name == &self.host)
            .or_else(|| booting.first())?;
        let boots = format!("{host}-boots");

        self.results
            .iter()
            .filter(|result| result.check == boots)
            .find_map(|result| result.boot_time?.total)
            .or_else(|| self.current_boot.get(host.as_str())?.total)
    }
}

fn checks(out_dir: &Path, out: &mut dyn Write) -> Result<(), BadgesError> {
    fs::create_dir_all(out_dir).map_err(io_error(&format!("create {}", out_dir.display())))?;

    for (name, badge) in Run::from_env().badges() {
        write_badge(&out_dir.join(name), &badge)?;
    }

    let mut names: Vec<String> = fs::read_dir(out_dir)
        .map_err(io_error(&format!("list {}", out_dir.display())))?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        let _ = writeln!(out, "{name}");
    }
    Ok(())
}

fn reproducibility_badge(summary: &str) -> Badge {
    let verdicts: Vec<Verdict> = summary
        .lines()
        .filter_map(|line| Some(Verdict::parse(line)?.1))
        .collect();
    let badge = |message: &str, color: &str| Badge::new("reproducibility", message, color);

    if verdicts.iter().any(|verdict| !verdict.finished()) {
        return badge("not finished", "lightgrey");
    }

    if !verdicts.iter().any(Verdict::rebuilt) {
        return badge("no host", "lightgrey");
    }

    match verdicts
        .iter()
        .filter(|verdict| **verdict == Verdict::Differs)
        .count()
    {
        0 => badge("reproducible", "brightgreen"),
        1 => badge("1 path differs", "orange"),
        differ => badge(&format!("{differ} paths differ"), "orange"),
    }
}

fn reproducibility(summary: &Path, out_dir: &Path, out: &mut dyn Write) -> Result<(), BadgesError> {
    fs::create_dir_all(out_dir).map_err(io_error(&format!("create {}", out_dir.display())))?;
    if !summary.is_file() {
        return Err(BadgesError::NoSummary(summary.display().to_string()));
    }

    let text =
        fs::read_to_string(summary).map_err(io_error(&format!("read {}", summary.display())))?;
    let written = write_badge(
        &out_dir.join("reproducibility.json"),
        &reproducibility_badge(&text),
    )?;
    let _ = write!(out, "{written}");
    Ok(())
}

fn publish(dir: &Path, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    match badges_branch::publish(&Machine, ".", dir, err) {
        Ok(published) => {
            let _ = writeln!(out, "ci-badges: {}", published.message());
            0
        }
        Err(error) => {
            let _ = writeln!(err, "ci-badges: {error}");
            FAILED
        }
    }
}

fn read(file: &str, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    match badges_branch::read(&Machine, ".", file) {
        Ok(text) => {
            let _ = write!(out, "{text}");
            0
        }
        Err(error) => {
            let _ = writeln!(err, "ci-badges: {error}");
            error.exit_code()
        }
    }
}

pub fn flaky_history(
    commands: &dyn Commands,
    repo: &str,
    err: &mut dyn Write,
) -> Result<FlakyHistory, FlakyHistoryError> {
    let read = badges_branch::read(commands, repo, FLAKY_FILE);
    if let Err(error) = &read {
        let _ = writeln!(err, "ci-badges: {error}");
        if matches!(error, ReadError::Fetch) {
            return Err(FlakyHistoryError::Fetch);
        }
    }

    if let Some(history) = json_or_none(read.ok().as_deref()) {
        return Ok(history);
    }

    let _ = writeln!(
        err,
        "flaky-history: no valid flaky.json on the badges branch, every VM check is measured"
    );
    Ok(FlakyHistory::default())
}
