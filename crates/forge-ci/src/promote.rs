mod signing;

use std::env;
use std::ffi::OsString;
use std::path::Path;

use forge::system::{Answer, Git, Machine};

use crate::badges_branch::GIT_TIMEOUT;
use crate::model::{HostChecks, JobStatus, PlannedJob, SkippedJob, Status, env_json, statuses};
use crate::results::check_results;
use signing::Signing;

const USAGE: &str = "usage: ci-promote <results-dir> <sha>";
const UNSIGNED: &str = "unsigned: DEPLOY_SIGNING_KEY is not set, so deploy branches point to main's own commits, which hosts with update.signers refuse";
const CANNOT_SIGN: &str =
    "DEPLOY_SIGNING_KEY cannot sign, it must hold an OpenSSH private key without a passphrase";
const NOT_A_FAST_FORWARD: &str = "not a fast-forward";
const DEPLOY_REFS: &str = "+refs/heads/deploy/*:refs/remotes/origin/deploy/*";
const USAGE_ERROR: u8 = 2;

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    Promote,
    Kept(String),
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let [results, sha] = args else {
        eprintln!("{USAGE}");
        return USAGE_ERROR;
    };

    let git = Git::new(&Machine, ".");
    let resolved = git.loud(&[
        "rev-parse",
        "--verify",
        &format!("{}^{{commit}}", sha.to_string_lossy()),
    ]);
    if !resolved.success {
        return resolved.exit_code();
    }

    let sha = resolved.line().to_owned();
    let Some(inputs) = Inputs::from_env() else {
        return USAGE_ERROR;
    };

    let decisions = decide(&inputs, Path::new(results));
    let fetched = git.fetch(&["-q", "--prune", "origin", DEPLOY_REFS], GIT_TIMEOUT);
    if !fetched.success {
        return fetched.exit_code();
    }

    let key = env::var_os("DEPLOY_SIGNING_KEY").filter(|key| !key.is_empty());
    let signing = match key
        .map(|key| Signing::new(&Machine, &git, &key))
        .transpose()
    {
        Ok(signing) => signing,
        Err(error) => {
            eprintln!("ci-promote: cannot write DEPLOY_SIGNING_KEY: {error}");
            return 1;
        }
    };
    if signing.is_none() {
        println!("{UNSIGNED}");
    }

    let promoter = Promoter {
        git: &git,
        short: short(&git, &sha),
        sha,
        signing: signing.as_ref(),
    };
    let mut refused = false;
    for (host, decision) in decisions {
        if let Decision::Kept(kept) = decision {
            println!("deploy/{host} kept: {kept}");
            continue;
        }

        if let Err(reason) = promoter.promote(&host) {
            refused = true;
            println!("deploy/{host} refused: {reason}");
        }
    }
    u8::from(refused)
}

struct Inputs {
    hosts: HostChecks,
    planned: Vec<PlannedJob>,
    skipped: Vec<SkippedJob>,
}

impl Inputs {
    fn from_env() -> Option<Self> {
        let hosts = required("HOST_CHECKS")?;
        let planned = required("PLANNED")?;
        let skipped = required("SKIPPED")?;
        Some(Self {
            hosts,
            planned,
            skipped,
        })
    }
}

fn required<T: serde::de::DeserializeOwned>(name: &str) -> Option<T> {
    let value = env_json(name);
    if value.is_none() {
        eprintln!("ci-promote: {name} must hold the {name} JSON of ci-plan");
    }
    value
}

fn decide(inputs: &Inputs, results: &Path) -> Vec<(String, Decision)> {
    let loaded = check_results(results);
    for unreadable in &loaded.unreadable {
        eprintln!(
            "ci-promote: {} is not a result: {}",
            unreadable.path.display(),
            unreadable.reason
        );
    }
    let statuses = statuses(&inputs.planned, &inputs.skipped, &loaded.values);
    inputs
        .hosts
        .iter()
        .map(|(host, checks)| (host.clone(), decision(checks, &statuses)))
        .collect()
}

fn decision(checks: &[String], statuses: &[JobStatus]) -> Decision {
    let mut kept: Vec<String> = checks
        .iter()
        .flat_map(|check| {
            let mut found: Vec<&Status> = statuses
                .iter()
                .filter(|job| job.check == check)
                .map(|job| &job.status)
                .collect();
            if found.is_empty() {
                found.push(&Status::DidNotFinish);
            }
            found
                .into_iter()
                .filter(|status| **status != Status::Passed)
                .map(move |status| format!("{check} {status}"))
        })
        .collect();
    if kept.is_empty() {
        return Decision::Promote;
    }

    kept.sort();
    kept.dedup();
    Decision::Kept(kept.join(", "))
}

fn short(git: &Git, commit: &str) -> String {
    git.quiet(&["rev-parse", "--short", commit])
        .line()
        .to_owned()
}

fn is_ancestor(git: &Git, ancestor: &str, descendant: &str) -> bool {
    git.quiet(&["merge-base", "--is-ancestor", ancestor, descendant])
        .success
}

fn subject_prefix(host: &str) -> String {
    format!("deploy {host}: checks passed for ")
}

fn is_promotion(git: &Git, host: &str, commit: &str) -> bool {
    git.quiet(&["log", "-1", "--format=%s", commit])
        .line()
        .starts_with(&subject_prefix(host))
}

struct Promoter<'a> {
    git: &'a Git<'a>,
    sha: String,
    short: String,
    signing: Option<&'a Signing>,
}

impl Promoter<'_> {
    fn promote(&self, host: &str) -> Result<(), String> {
        let tip = self
            .git
            .quiet(&[
                "rev-parse",
                "-q",
                "--verify",
                &format!("refs/remotes/origin/deploy/{host}"),
            ])
            .line()
            .to_owned();
        let tip = Some(tip).filter(|tip| !tip.is_empty());
        match self.signing {
            None => self.promote_unsigned(host, tip.as_deref()),
            Some(signing) => self.promote_signed(signing, host, tip.as_deref()),
        }
    }

    fn already(&self, host: &str) {
        println!("deploy/{host} already at or past {}", self.short);
    }

    fn promote_unsigned(&self, host: &str, tip: Option<&str>) -> Result<(), String> {
        if tip.is_some_and(|tip| is_ancestor(self.git, &self.sha, tip)) {
            self.already(host);
            return Ok(());
        }

        self.push(host, &self.sha, None)?;
        println!("deploy/{host} -> {}", self.short);
        Ok(())
    }

    fn promote_signed(
        &self,
        signing: &Signing,
        host: &str,
        tip: Option<&str>,
    ) -> Result<(), String> {
        let previous = match tip {
            Some(tip) if !signing.verified(self.git, host, tip) => {
                warn(&format!(
                    "deploy/{host} warning: its tip {} is not a promotion signed by DEPLOY_SIGNING_KEY, so the promotion replaces it instead of building on it",
                    short(self.git, tip)
                ));
                None
            }
            other => other,
        };
        let lease = match (tip, previous) {
            (Some(tip), None) => Some(tip),
            _ => None,
        };
        if let Some(previous) = previous {
            let carried = self.carried(host, previous);
            if is_ancestor(self.git, &self.sha, &carried) {
                self.already(host);
                return Ok(());
            }
            if !is_ancestor(self.git, &carried, &self.sha) {
                return Err(NOT_A_FAST_FORWARD.to_owned());
            }
        }

        let target = self
            .promotion(signing, host, previous)
            .ok_or_else(|| CANNOT_SIGN.to_owned())?;
        self.push(host, &target, lease)?;
        println!(
            "deploy/{host} -> {}, signed as {}",
            self.short,
            short(self.git, &target)
        );
        Ok(())
    }

    fn carried(&self, host: &str, commit: &str) -> String {
        if !is_promotion(self.git, host, commit) {
            return commit.to_owned();
        }

        let parents = self
            .git
            .quiet(&["rev-list", "--parents", "-n", "1", commit]);
        parents
            .line()
            .split_whitespace()
            .next_back()
            .unwrap_or(commit)
            .to_owned()
    }

    fn promotion(&self, signing: &Signing, host: &str, previous: Option<&str>) -> Option<String> {
        let mut args: Vec<String> = signing.config().to_vec();
        args.extend(["commit-tree".to_owned(), "-S".to_owned()]);
        if let Some(previous) =
            previous.filter(|previous| !is_ancestor(self.git, previous, &self.sha))
        {
            args.extend(["-p".to_owned(), previous.to_owned()]);
        }
        args.extend([
            "-p".to_owned(),
            self.sha.clone(),
            "-m".to_owned(),
            format!("{}{}", subject_prefix(host), self.short),
            format!("{}^{{tree}}", self.sha),
        ]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let created = self.git.quiet(&args);
        created
            .success
            .then(|| created.line().to_owned())
            .filter(|target| !target.is_empty())
    }

    fn push(&self, host: &str, target: &str, lease: Option<&str>) -> Result<(), String> {
        let lease = lease.map(|tip| format!("--force-with-lease=refs/heads/deploy/{host}:{tip}"));
        let refspec = format!("{target}:refs/heads/deploy/{host}");
        let mut args = vec!["push", "--porcelain"];
        args.extend(lease.as_deref());
        args.extend(["origin", &refspec]);
        let pushed = self
            .git
            .run(&self.git.invocation(&args).timeout(GIT_TIMEOUT));
        if pushed.success {
            return Ok(());
        }
        Err(refusal(&pushed))
    }
}

fn refusal(pushed: &Answer) -> String {
    let output = format!("{}{}", pushed.stdout, pushed.stderr);
    let reason = output
        .lines()
        .find(|line| line.starts_with('!'))
        .and_then(|line| line.split('\t').nth(2))
        .unwrap_or_default();
    if ["non-fast-forward", "fetch first", "stale info"]
        .iter()
        .any(|cause| reason.contains(cause))
    {
        return NOT_A_FAST_FORWARD.to_owned();
    }
    if !reason.is_empty() {
        return reason.to_owned();
    }

    output
        .lines()
        .rfind(|line| *line != "Done")
        .unwrap_or_default()
        .to_owned()
}

fn warn(message: &str) {
    println!("{message}");
    if env::var_os("GITHUB_ACTIONS").is_some_and(|value| !value.is_empty()) {
        println!("::warning::{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(stdout: &str, stderr: &str) -> Answer {
        Answer {
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
            ..Answer::failed("")
        }
    }

    #[test]
    fn a_rejected_ref_gives_its_summary_and_a_stale_lease_is_not_a_fast_forward() {
        let rejected = answer(
            "To ../origin.git\n!\tabc:refs/heads/deploy/x\t[remote rejected] (pre-receive hook declined)\nDone\n",
            "error: failed to push some refs\n",
        );
        assert_eq!(
            refusal(&rejected),
            "[remote rejected] (pre-receive hook declined)"
        );
        for summary in [
            "[rejected] (non-fast-forward)",
            "[rejected] (fetch first)",
            "[rejected] (stale info)",
        ] {
            let stale = answer(&format!("!\ta:b\t{summary}\nDone\n"), "");
            assert_eq!(refusal(&stale), NOT_A_FAST_FORWARD, "{summary}");
        }
    }

    #[test]
    fn without_a_rejected_ref_the_last_line_but_done_is_the_reason() {
        let unreachable = answer(
            "",
            "fatal: '../gone.git' does not appear to be a git repository\nDone\n",
        );
        assert_eq!(
            refusal(&unreachable),
            "fatal: '../gone.git' does not appear to be a git repository"
        );
    }
}
