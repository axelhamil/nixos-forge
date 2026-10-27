use std::ffi::OsString;
use std::io::{self, Write};

use forge::system::{Commands, Git, Machine};
use serde::{Deserialize, Serialize};

use super::RUNS;
use crate::badges::{FlakyHistoryError, flaky_history};
use crate::jobs::{self, JobsError};
use crate::model::FlakyHistory;

const USAGE: &str = "usage: ci-flaky-plan";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlakyJob {
    pub check: String,
    pub system: String,
    pub runner: String,
    #[serde(default, rename = "drvPath", skip_serializing_if = "Option::is_none")]
    pub drv_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub checks: Vec<FlakyJob>,
    pub all: Vec<FlakyJob>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error(transparent)]
    History(#[from] FlakyHistoryError),
    #[error("ci-flaky-plan: git rev-parse HEAD^{{tree}} failed: {0}")]
    Tree(String),
    #[error("ci-flaky-plan: {0}")]
    Jobs(JobsError),
    #[error("{0}")]
    DoesNotEvaluate(JobsError),
}

impl From<JobsError> for PlanError {
    fn from(error: JobsError) -> Self {
        match error {
            JobsError::DoesNotEvaluate(_) => Self::DoesNotEvaluate(error),
            other => Self::Jobs(other),
        }
    }
}

pub(crate) fn tree(commands: &dyn Commands) -> Result<String, String> {
    let answer = Git::new(commands, ".").quiet(&["rev-parse", "HEAD^{tree}"]);
    if !answer.success {
        return Err(answer.stderr.trim().to_owned());
    }
    Ok(answer.line().to_owned())
}

fn measured(history: &FlakyHistory, job: &FlakyJob) -> bool {
    let Some(record) = job
        .drv_path
        .as_ref()
        .and_then(|drv| history.checks.get(drv))
    else {
        return false;
    };
    record.runs.len() >= RUNS as usize && record.runs.iter().all(|run| run.passed)
}

pub fn plan(commands: &dyn Commands, err: &mut dyn Write) -> Result<Plan, PlanError> {
    let history = flaky_history(commands, ".", err)?;
    let tree = tree(commands).map_err(PlanError::Tree)?;
    if history.tree.as_deref() == Some(tree.as_str()) {
        let _ = writeln!(
            err,
            "ci-flaky-plan: this tree was already measured and every VM check passed all of its runs"
        );
        return Ok(Plan::default());
    }

    let jobs: Vec<FlakyJob> = jobs::ci(commands, "flakyMatrix")?;
    let evaluated = jobs::evaluate(commands, false, jobs::workers())?;

    let all = jobs
        .into_iter()
        .map(|job| {
            let found = jobs::find(&evaluated, &job.system, &job.check)?;
            let drv_path = found
                .drv_path
                .clone()
                .ok_or_else(|| JobsError::Missing(found.attr.clone()))?;
            Ok(FlakyJob {
                drv_path: Some(drv_path),
                runs: None,
                ..job
            })
        })
        .collect::<Result<Vec<_>, JobsError>>()?;

    let checks = all
        .iter()
        .filter(|job| !measured(&history, job))
        .map(|job| FlakyJob {
            runs: Some(RUNS),
            ..job.clone()
        })
        .collect();
    Ok(Plan { checks, all })
}

fn compact<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_owned())
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    match args.first() {
        Some(arg) if arg == "-h" || arg == "--help" => {
            println!("{USAGE}");
            return 0;
        }
        Some(arg) if !arg.is_empty() => {
            eprintln!("{USAGE}");
            return 2;
        }
        _ => {}
    }

    match plan(&Machine, &mut io::stderr()) {
        Ok(plan) => {
            println!("checks={}", compact(&plan.checks));
            println!("all={}", compact(&plan.all));
            0
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}
