use std::path::Path;
use std::time::Duration;

use super::config::{Config, Operation};
use crate::system::{Commands, Files, Invocation};

const EVAL_TIMEOUT: Duration = Duration::from_mins(30);
const DIFF_TIMEOUT: Duration = Duration::from_mins(10);
const CACHE_MISSES: [&str; 2] = ["local builds are disabled", "Unable to start any build"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    CacheLacksVersion,
    Failed,
}

fn target(repo: &str, to: &str) -> String {
    format!("git+file://{repo}?rev={to}")
}

#[must_use]
pub fn rebuild(config: &Config, repo: &str, to: &str, variant: Option<&str>) -> Invocation {
    let flake = format!("{}#{}", target(repo, to), config.host);
    let mut invocation = Invocation::new(
        &config.rebuild_command,
        &[config.operation.as_str(), "--flake", &flake],
    );
    if !config.allow_local_build {
        invocation.args.extend(
            [
                "--option",
                "max-jobs",
                "0",
                "--option",
                "always-allow-substitutes",
                "true",
            ]
            .map(str::to_owned),
        );
    }
    if let Some(variant) = variant {
        invocation
            .args
            .extend(["--specialisation".to_owned(), variant.to_owned()]);
    }
    invocation
}

#[must_use]
pub fn kept_variant(
    commands: &dyn Commands,
    files: &dyn Files,
    config: &Config,
    repo: &str,
    to: &str,
    running_variant: &Path,
) -> Option<String> {
    if config.operation == Operation::Boot {
        return None;
    }
    let text = String::from_utf8(files.read(running_variant).ok()?).ok()?;
    if text.is_empty() {
        return None;
    }
    let variant = text.trim_end_matches('\n');

    let attribute = format!(
        "{}#nixosConfigurations.{}.config.specialisation",
        target(repo, to),
        config.host
    );
    let answer = commands.run(
        &Invocation::new(
            "nix",
            &[
                "eval",
                "--json",
                &attribute,
                "--apply",
                "builtins.attrNames",
            ],
        )
        .timeout(EVAL_TIMEOUT)
        .loud(),
    );
    if !answer.success {
        return None;
    }
    let variants: Vec<String> = serde_json::from_str(&answer.stdout).ok()?;
    variants
        .iter()
        .any(|name| name == variant)
        .then(|| variant.to_owned())
}

#[must_use]
pub fn classify(log: &str) -> Failure {
    if CACHE_MISSES.iter().any(|miss| log.contains(miss)) {
        return Failure::CacheLacksVersion;
    }
    Failure::Failed
}

#[must_use]
pub fn closure_changes(commands: &dyn Commands, old: &Path, new: &Path) -> String {
    let old = old.display().to_string();
    let new = new.display().to_string();
    let answer = commands.run(&Invocation::new("nvd", &["diff", &old, &new]).timeout(DIFF_TIMEOUT));
    answer
        .stdout
        .lines()
        .filter(|line| line.starts_with("Closure size:"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::system::{Answer, Machine};

    fn config(operation: Operation, allow_local_build: bool) -> Config {
        Config {
            host: "machine".to_owned(),
            state_dir: PathBuf::from("/state"),
            checkout: None,
            sources: Vec::new(),
            operation,
            rebuild_command: "/bin/nixos-rebuild".to_owned(),
            allow_local_build,
            allowed_signers: None,
        }
    }

    struct Eval(&'static str);

    impl Commands for Eval {
        fn run(&self, invocation: &Invocation) -> Answer {
            assert_eq!(
                invocation.command_line(),
                "nix eval --json git+file:///repo?rev=to#nixosConfigurations.machine.config.specialisation --apply builtins.attrNames"
            );
            Answer::ok(self.0)
        }
    }

    fn running_variant(name: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("specialisation");
        fs::write(&path, name).unwrap();
        (dir, path)
    }

    #[test]
    fn without_local_builds_the_rebuild_only_substitutes() {
        let invocation = rebuild(&config(Operation::Boot, false), "/repo", "abc", None);

        assert_eq!(
            invocation.command_line(),
            "/bin/nixos-rebuild boot --flake git+file:///repo?rev=abc#machine --option max-jobs 0 --option always-allow-substitutes true"
        );
    }

    #[test]
    fn a_switch_keeps_the_running_variant_when_the_new_version_still_has_it() {
        let (_dir, path) = running_variant("gaming\n");
        let switch = config(Operation::Switch, true);

        let kept = kept_variant(
            &Eval(r#"["gaming","work"]"#),
            &Machine,
            &switch,
            "/repo",
            "to",
            &path,
        );
        assert_eq!(kept.as_deref(), Some("gaming"));
        assert_eq!(
            rebuild(&switch, "/repo", "to", kept.as_deref()).command_line(),
            "/bin/nixos-rebuild switch --flake git+file:///repo?rev=to#machine --specialisation gaming"
        );
        assert_eq!(
            kept_variant(
                &Eval(r#"["work"]"#),
                &Machine,
                &switch,
                "/repo",
                "to",
                &path
            ),
            None
        );
        assert_eq!(
            kept_variant(
                &Eval(r#"["gaming"]"#),
                &Machine,
                &config(Operation::Boot, true),
                "/repo",
                "to",
                &path
            ),
            None
        );
    }

    #[test]
    fn a_missing_cache_entry_waits_and_anything_else_fails() {
        assert_eq!(
            classify("error: you're trying to build, but local builds are disabled\n"),
            Failure::CacheLacksVersion
        );
        assert_eq!(
            classify("error: Unable to start any build; either increase '--max-jobs'\n"),
            Failure::CacheLacksVersion
        );
        assert_eq!(
            classify("error: attribute 'machine' missing\n"),
            Failure::Failed
        );
    }
}
