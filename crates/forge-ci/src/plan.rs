use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use forge::system::{Commands, Machine};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::jobs::{self, Evaluated, JobsError};
use crate::model::{CACHED, COMPILES_A_KERNEL, PlannedJob, SkippedJob};

const USAGE: &str = "usage: ci-plan <event>";
const KERNEL_TIMEOUT: u32 = 240;
const TIMEOUT: u32 = 60;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Jobs(#[from] JobsError),
    #[error("cannot write the summary to {}: {source}", path.display())]
    Summary { path: PathBuf, source: io::Error },
    #[error("cannot print the plan: {0}")]
    Print(io::Error),
}

#[derive(Debug, Clone, Deserialize)]
struct MatrixJob {
    check: String,
    system: String,
    #[serde(default)]
    runner: String,
    #[serde(default)]
    vm: bool,
    #[serde(default, rename = "baseInput")]
    base_input: Option<String>,
}

struct Status {
    cached: bool,
    kernel: bool,
}

struct Job {
    planned: PlannedJob,
    cached: bool,
    kernel: bool,
    build: bool,
}

struct Plan {
    checks: Vec<PlannedJob>,
    skipped: Vec<SkippedJob>,
    closures: Vec<Map<String, Value>>,
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let Some(event) = args.first().and_then(|event| event.to_str()) else {
        eprintln!("{USAGE}");
        return 2;
    };

    let summary = env::var_os("GITHUB_STEP_SUMMARY")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    match print_plan(&Machine, event, summary.as_deref(), &mut io::stdout()) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("ci-plan: {error}");
            1
        }
    }
}

fn print_plan(
    commands: &dyn Commands,
    event: &str,
    summary: Option<&Path>,
    out: &mut dyn Write,
) -> Result<(), Error> {
    let kernels: BTreeMap<String, Vec<String>> = jobs::ci(commands, "kernels")?;
    let matrix: Vec<MatrixJob> = jobs::ci(commands, "matrix")?;
    let closures: Vec<Map<String, Value>> = jobs::ci(commands, "closureMatrix")?;
    let perf: Value = jobs::ci(commands, "perfMatrix")?;
    let host_checks: Value = jobs::ci(commands, "hostChecks")?;
    let descriptions: Value = jobs::ci(commands, "descriptions")?;

    let evaluated = jobs::evaluate(commands, true, jobs::workers())?;
    let plan = plan(event, &matrix, &closures, &kernels, &evaluated)?;

    if let Some(path) = summary {
        append(path, &summary_of(&plan)).map_err(|source| Error::Summary {
            path: path.to_path_buf(),
            source,
        })?;
    }

    let lines = [
        ("checks", json(&plan.checks)),
        ("skipped", json(&plan.skipped)),
        ("closures", json(&plan.closures)),
        ("perf", json(&perf)),
        ("hostChecks", json(&host_checks)),
        ("descriptions", json(&descriptions)),
    ];
    let mut text = String::new();
    for (key, value) in lines {
        let _ = writeln!(text, "{key}={value}");
    }
    out.write_all(text.as_bytes()).map_err(Error::Print)
}

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
}

fn append(path: &Path, text: &str) -> io::Result<()> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(text.as_bytes())
}

fn status(evaluated: &Evaluated, kernels: &BTreeMap<String, Vec<String>>) -> Status {
    let system = evaluated.attr.split('.').next().unwrap_or_default();
    let kernel = kernels.get(system).is_some_and(|kernels| {
        evaluated
            .needed_builds
            .iter()
            .any(|needed| kernels.contains(needed))
    });
    Status {
        cached: evaluated.is_cached,
        kernel,
    }
}

fn plan(
    event: &str,
    matrix: &[MatrixJob],
    closures: &[Map<String, Value>],
    kernels: &BTreeMap<String, Vec<String>>,
    evaluated: &[Evaluated],
) -> Result<Plan, JobsError> {
    let jobs = matrix
        .iter()
        .map(|job| {
            let status = status(jobs::find(evaluated, &job.system, &job.check)?, kernels);
            let cached = status.cached && job.base_input.is_none();
            let build = !cached && (event == "push" || !status.kernel);
            Ok(Job {
                planned: PlannedJob {
                    check: job.check.clone(),
                    system: job.system.clone(),
                    runner: job.runner.clone(),
                    vm: job.vm,
                    base_input: job.base_input.clone(),
                    timeout: if status.kernel {
                        KERNEL_TIMEOUT
                    } else {
                        TIMEOUT
                    },
                },
                cached,
                kernel: status.kernel,
                build,
            })
        })
        .collect::<Result<Vec<_>, JobsError>>()?;

    let closures = closures
        .iter()
        .map(|closure| {
            let system = closure.get("system").and_then(Value::as_str);
            let compiles_a_kernel = jobs.iter().any(|job| {
                Some(job.planned.system.as_str()) == system && job.kernel && !job.cached
            });
            let mut closure = closure.clone();
            closure.insert("build".to_owned(), Value::Bool(!compiles_a_kernel));
            closure
        })
        .collect();

    let skipped = jobs
        .iter()
        .filter(|job| !job.build)
        .map(|job| SkippedJob {
            check: job.planned.check.clone(),
            system: job.planned.system.clone(),
            vm: job.planned.vm,
            reason: if job.cached {
                CACHED
            } else {
                COMPILES_A_KERNEL
            }
            .to_owned(),
        })
        .collect();
    let checks = jobs
        .into_iter()
        .filter(|job| job.build)
        .map(|job| job.planned)
        .collect();

    Ok(Plan {
        checks,
        skipped,
        closures,
    })
}

fn summary_of(plan: &Plan) -> String {
    let mut summary = format!("### Plan\n\n{} checks to build.\n", plan.checks.len());
    for job in &plan.skipped {
        let _ = writeln!(
            summary,
            "- `{}` ({}): {}",
            job.check, job.system, job.reason
        );
    }
    summary
}
