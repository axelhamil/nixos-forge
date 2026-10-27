use std::thread;
use std::time::Duration;

use forge::system::{Commands, Invocation};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::model::key;

pub const EVALUATION_TIMEOUT: Duration = Duration::from_mins(90);

const MAX_MEMORY_SIZE: &str = "3072";

pub const EVALUATION_OUTPUT_LIMIT: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Evaluated {
    pub attr: String,
    #[serde(default, rename = "isCached")]
    pub is_cached: bool,
    #[serde(default, rename = "neededBuilds")]
    pub needed_builds: Vec<String>,
    #[serde(default, rename = "drvPath")]
    pub drv_path: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unevaluated {
    pub attr: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JobsError {
    #[error("{command} failed")]
    Failed { command: String },
    #[error("{command} printed invalid JSON: {reason}")]
    Invalid { command: String, reason: String },
    #[error("{}", lines(.0))]
    DoesNotEvaluate(Vec<Unevaluated>),
    #[error("{0} is missing from the evaluation")]
    Missing(String),
}

fn lines(unevaluated: &[Unevaluated]) -> String {
    unevaluated
        .iter()
        .map(|job| format!("{} does not evaluate: {}", job.attr, job.error))
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn workers() -> usize {
    thread::available_parallelism().map_or(1, usize::from)
}

pub fn ci<T: DeserializeOwned>(commands: &dyn Commands, attr: &str) -> Result<T, JobsError> {
    let installable = format!(".#ci.{attr}");
    let invocation = Invocation::new("nix", &["eval", "--json", &installable])
        .timeout(EVALUATION_TIMEOUT)
        .loud();
    let stdout = output(commands, &invocation)?;
    serde_json::from_str(&stdout).map_err(|error| JobsError::Invalid {
        command: invocation.command_line(),
        reason: error.to_string(),
    })
}

fn output(commands: &dyn Commands, invocation: &Invocation) -> Result<String, JobsError> {
    let answer = commands.run(invocation);
    if !answer.success {
        return Err(JobsError::Failed {
            command: invocation.command_line(),
        });
    }
    Ok(answer.stdout)
}

pub fn evaluate(
    commands: &dyn Commands,
    cache_status: bool,
    workers: usize,
) -> Result<Vec<Evaluated>, JobsError> {
    let workers = workers.to_string();
    let mut args = vec!["--flake", ".#checks", "--force-recurse"];
    if cache_status {
        args.push("--check-cache-status");
    }
    args.extend(["--workers", &workers, "--max-memory-size", MAX_MEMORY_SIZE]);
    let invocation = Invocation::new("nix-eval-jobs", &args)
        .timeout(EVALUATION_TIMEOUT)
        .output_limit(EVALUATION_OUTPUT_LIMIT)
        .loud();
    let stdout = output(commands, &invocation)?;

    let evaluated = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<Evaluated>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| JobsError::Invalid {
            command: invocation.command_line(),
            reason: error.to_string(),
        })?;

    let unevaluated: Vec<Unevaluated> = evaluated
        .iter()
        .filter_map(|job| {
            Some(Unevaluated {
                attr: job.attr.clone(),
                error: job.error.clone()?,
            })
        })
        .collect();
    if !unevaluated.is_empty() {
        return Err(JobsError::DoesNotEvaluate(unevaluated));
    }
    Ok(evaluated)
}

pub fn find<'a>(
    evaluated: &'a [Evaluated],
    system: &str,
    check: &str,
) -> Result<&'a Evaluated, JobsError> {
    let attr = key(system, check);
    evaluated
        .iter()
        .rev()
        .find(|job| job.attr == attr)
        .ok_or(JobsError::Missing(attr))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use forge::system::Answer;

    use super::*;

    struct Nix {
        eval_jobs: Answer,
        calls: RefCell<Vec<Invocation>>,
    }

    impl Nix {
        fn printing(stdout: &str) -> Self {
            Self {
                eval_jobs: Answer::ok(stdout),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn command_lines(&self) -> Vec<String> {
            self.calls
                .borrow()
                .iter()
                .map(Invocation::command_line)
                .collect()
        }
    }

    impl Commands for Nix {
        fn run(&self, invocation: &Invocation) -> Answer {
            self.calls.borrow_mut().push(invocation.clone());
            match invocation.command_line().as_str() {
                "nix eval --json .#ci.hostChecks" => Answer::ok(r#"{"example":["example-boots"]}"#),
                "nix eval --json .#ci.broken" => {
                    Answer::failed("error: attribute 'broken' missing")
                }
                "nix eval --json .#ci.garbled" => Answer::ok("{"),
                line if line.starts_with("nix-eval-jobs ") => self.eval_jobs.clone(),
                _ => Answer::failed("unexpected"),
            }
        }
    }

    const EVALUATED: &str = r#"{"attr":"x86_64-linux.example-toplevel","isCached":false,"neededBuilds":["/nix/store/x-linux-6.12.drv"],"drvPath":"/nix/store/t.drv"}

{"attr":"x86_64-linux.docs","isCached":true,"neededBuilds":[],"name":"docs","system":"x86_64-linux"}
"#;

    #[test]
    fn a_ci_attribute_is_read_as_json() {
        let nix = Nix::printing("");
        let hosts: crate::model::HostChecks = ci(&nix, "hostChecks").unwrap();
        assert_eq!(hosts["example"], ["example-boots"]);
        assert!(nix.calls.borrow()[0].timeout.is_some());
    }

    #[test]
    fn a_ci_attribute_that_fails_or_is_not_json_is_an_error() {
        let nix = Nix::printing("");
        assert_eq!(
            ci::<serde_json::Value>(&nix, "broken")
                .unwrap_err()
                .to_string(),
            "nix eval --json .#ci.broken failed"
        );
        assert!(matches!(
            ci::<serde_json::Value>(&nix, "garbled"),
            Err(JobsError::Invalid { .. })
        ));
    }

    #[test]
    fn the_checks_are_evaluated_with_the_cache_status_when_asked() {
        let nix = Nix::printing(EVALUATED);

        let evaluated = evaluate(&nix, true, 4).unwrap();
        evaluate(&nix, false, 2).unwrap();

        assert_eq!(
            nix.command_lines(),
            [
                "nix-eval-jobs --flake .#checks --force-recurse --check-cache-status --workers 4 --max-memory-size 3072",
                "nix-eval-jobs --flake .#checks --force-recurse --workers 2 --max-memory-size 3072",
            ]
        );
        assert!(
            nix.calls
                .borrow()
                .iter()
                .all(|call| call.output_limit == Some(EVALUATION_OUTPUT_LIMIT))
        );
        assert_eq!(evaluated.len(), 2);
        assert!(!evaluated[0].is_cached);
        assert_eq!(evaluated[0].needed_builds, ["/nix/store/x-linux-6.12.drv"]);
        assert_eq!(evaluated[0].drv_path.as_deref(), Some("/nix/store/t.drv"));
        assert!(evaluated[1].is_cached);
    }

    #[test]
    fn every_check_that_does_not_evaluate_is_named_with_its_error() {
        let nix = Nix::printing(
            r#"{"attr":"x86_64-linux.example-toplevel","isCached":false,"neededBuilds":[]}
{"attr":"x86_64-linux.example-boots","error":"error: undefined variable 'foo'"}
{"attr":"aarch64-linux.example-pi-sd-image","error":"error: infinite recursion"}
"#,
        );

        let error = evaluate(&nix, true, 1).unwrap_err();

        assert_eq!(
            error.to_string(),
            "x86_64-linux.example-boots does not evaluate: error: undefined variable 'foo'\naarch64-linux.example-pi-sd-image does not evaluate: error: infinite recursion"
        );
    }

    #[test]
    fn a_failed_or_garbled_evaluation_is_an_error() {
        let failed = Nix {
            eval_jobs: Answer::failed("killed"),
            calls: RefCell::new(Vec::new()),
        };
        assert!(matches!(
            evaluate(&failed, true, 1),
            Err(JobsError::Failed { .. })
        ));
        assert!(matches!(
            evaluate(&Nix::printing("{\"attr\":1}\n"), true, 1),
            Err(JobsError::Invalid { .. })
        ));
    }

    #[test]
    fn a_job_is_found_by_its_system_and_check() {
        let evaluated = evaluate(&Nix::printing(EVALUATED), true, 1).unwrap();
        assert!(find(&evaluated, "x86_64-linux", "docs").unwrap().is_cached);
        assert_eq!(
            find(&evaluated, "aarch64-linux", "docs")
                .unwrap_err()
                .to_string(),
            "aarch64-linux.docs is missing from the evaluation"
        );
    }
}
