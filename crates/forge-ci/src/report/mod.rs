mod causes;
mod sections;
mod truncate;

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::model::{Descriptions, HostChecks, env_json, json_or_none};
use crate::results;
use causes::Causes;

const USAGE: &str = "usage: ci-report [--comment]";
const USAGE_ERROR: u8 = 2;
const FAILURE: u8 = 1;

const SUMMARY_LIMIT: usize = 1_048_576;
const COMMENT_LIMIT: usize = 65_536;
const SUMMARY_FOOTER: &str =
    "The report is too long for the run summary: the end is cut, the build logs hold every detail.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Summary,
    Comment,
}

impl Mode {
    fn parse(args: &[OsString]) -> Option<Self> {
        match args.first().map(|arg| arg.to_str()) {
            None | Some(Some("")) => Some(Self::Summary),
            Some(Some("--comment")) => Some(Self::Comment),
            Some(_) => None,
        }
    }

    fn limit(self) -> usize {
        match self {
            Self::Summary => SUMMARY_LIMIT,
            Self::Comment => COMMENT_LIMIT,
        }
    }

    fn footer(self, run: &str) -> String {
        match self {
            Self::Summary => SUMMARY_FOOTER.to_owned(),
            Self::Comment => format!(
                "The report is too long for a comment: read it in full in the [run summary]({run})."
            ),
        }
    }
}

#[derive(Debug, Default)]
pub struct Inputs {
    pub planned: Option<Vec<Value>>,
    pub skipped: Vec<Value>,
    pub hosts: HostChecks,
    pub descriptions: Descriptions,
    pub base_boot: Map<String, Value>,
    pub results: Vec<Map<String, Value>>,
    pub perf: BTreeMap<String, Map<String, Value>>,
    pub cache: String,
    pub lint: String,
    pub plan: String,
    pub run: String,
}

fn text_env(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

fn results_dir() -> PathBuf {
    let dir = text_env("RESULTS_DIR");
    if dir.is_empty() {
        return PathBuf::from("results");
    }

    PathBuf::from(dir)
}

fn objects(dir: &Path) -> Vec<Map<String, Value>> {
    results::load::<Value>(dir)
        .values
        .into_iter()
        .filter_map(|value| match value {
            Value::Object(result) => Some(result),
            _ => None,
        })
        .collect()
}

fn closures(dir: &Path) -> String {
    results::files_with(dir, "md")
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("closure-"))
        })
        .filter_map(|file| fs::read(file).ok())
        .fold(String::new(), |mut closures, bytes| {
            closures.push_str(&String::from_utf8_lossy(&bytes));
            closures.push('\n');
            closures
        })
}

impl Inputs {
    fn from_env(dir: &Path) -> Self {
        let base_boot = fs::read_to_string(dir.join("base/boot-time.json")).ok();
        Self {
            planned: env_json("PLANNED"),
            skipped: env_json("SKIPPED").unwrap_or_default(),
            hosts: env_json("HOST_CHECKS").unwrap_or_default(),
            descriptions: env_json("DESCRIPTIONS").unwrap_or_default(),
            base_boot: json_or_none(base_boot.as_deref()).unwrap_or_default(),
            results: objects(dir),
            perf: results::perf(dir),
            cache: text_env("CACHE_OUTCOME"),
            lint: text_env("LINT_RESULT"),
            plan: text_env("PLAN_RESULT"),
            run: text_env("RUN_URL"),
        }
    }
}

fn with_closures(mut report: String, closures: &str) -> String {
    if closures.is_empty() {
        return report;
    }

    report.push_str("<details>\n<summary>Package changes per machine</summary>\n\n");
    report.push_str(closures);
    report.push_str("</details>\n");
    report
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let Some(mode) = Mode::parse(args) else {
        eprintln!("{USAGE}");
        return USAGE_ERROR;
    };

    let causes = match Causes::embedded() {
        Ok(causes) => causes,
        Err(error) => {
            eprintln!("ci-report: {error}");
            return FAILURE;
        }
    };

    let dir = results_dir();
    let inputs = Inputs::from_env(&dir);
    let report = with_closures(sections::render(&inputs, &causes), &closures(&dir));
    let report = truncate::truncate(&report, mode.limit(), &mode.footer(&inputs.run));
    if let Err(error) = io::stdout().lock().write_all(report.as_bytes()) {
        eprintln!("ci-report: cannot write the report: {error}");
        return FAILURE;
    }

    0
}
