mod common;

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Output;

use forge_ci::excerpt::excerpt;
use serde_json::{Value, json};
use tempfile::TempDir;

const LOGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/logs");

const FAKE_NIX: &str = r#"#!/bin/sh
echo "$*" >> "$NIX_CALLS"
[ -z "$LOG" ] || cat "$LOG" >&2
[ -z "$OUT_PATH" ] || echo "$OUT_PATH"
[ -z "$ERR_PATH" ] || echo "$ERR_PATH" >&2
exit "${STATUS:-0}"
"#;

const BUILD_LINE: &str = "build --no-link --print-build-logs --print-out-paths";

fn log(name: &str) -> PathBuf {
    Path::new(LOGS).join(name)
}

fn excerpt_of(text: &str) -> Vec<String> {
    let lines = excerpt(text.as_bytes()).unwrap();
    assert!(lines.len() <= 20, "more than 20 lines: {lines:?}");
    assert!(
        lines.iter().all(|line| !line.contains('\u{1b}')),
        "terminal escapes kept: {lines:?}"
    );
    let unique: HashSet<&String> = lines.iter().collect();
    assert_eq!(unique.len(), lines.len(), "a line repeats: {lines:?}");
    lines
}

fn excerpt_of_log(name: &str) -> Vec<String> {
    excerpt_of(&fs::read_to_string(log(name)).unwrap())
}

fn has_line(lines: &[String], matches: impl Fn(&str) -> bool) -> bool {
    lines.iter().any(|line| matches(line))
}

#[test]
fn an_assertion_keeps_the_error_and_its_traceback() {
    let lines = excerpt_of_log("assertion.log");
    assert!(
        has_line(&lines, |line| line
            == "AssertionError: doctor misses a file written to /"),
        "{lines:?}"
    );
    assert!(
        has_line(&lines, |line| line == "Traceback (most recent call last):"),
        "{lines:?}"
    );
}

#[test]
fn an_evaluation_error_keeps_the_nix_error() {
    let lines = excerpt_of_log("eval-error.log");
    assert!(
        has_line(&lines, |line| line == "error: undefined variable 'foo'"),
        "{lines:?}"
    );
}

#[test]
fn a_timeout_keeps_the_failed_action() {
    let lines = excerpt_of_log("timeout.log");
    assert!(
        has_line(&lines, |line| line.starts_with(
            "RequestedAssertionFailed: action timed out after 120.73 seconds"
        )),
        "{lines:?}"
    );
}

#[test]
fn a_late_failure_survives_a_noisy_log_of_matching_lines() {
    let mut noisy = String::new();
    for index in 1..=30 {
        writeln!(
            noisy,
            "machine # [{index}] dhcpcd: eth0: DHCPv6 solicit timed out"
        )
        .unwrap();
    }
    noisy.push_str("AssertionError: late failure\n");

    let lines = excerpt_of(&noisy);

    assert!(
        has_line(&lines, |line| line == "AssertionError: late failure"),
        "{lines:?}"
    );
}

#[test]
fn a_compile_error_keeps_the_compiler_and_the_nix_errors() {
    let lines = excerpt_of_log("compile-error.log");
    assert!(
        has_line(&lines, |line| line
            == "main.c:3:1: error: expected ';' before '}' token"),
        "{lines:?}"
    );
    assert!(
        has_line(&lines, |line| line.starts_with("error: Cannot build ")),
        "{lines:?}"
    );
}

#[test]
fn a_failed_substitution_keeps_the_network_error() {
    let lines = excerpt_of_log("substitute.log");
    assert!(
        has_line(&lines, |line| line.starts_with(
            "error: some substitutes for the outputs of derivation "
        ) && line
            .contains(" failed (usually happens due to networking issues)")),
        "{lines:?}"
    );
}

#[test]
fn a_log_without_failure_yields_no_excerpt() {
    let mut quiet = String::new();
    for index in 1..=5000 {
        writeln!(quiet, "machine # line {index} of a healthy boot").unwrap();
    }
    assert!(excerpt_of(&quiet).is_empty());
}

#[test]
fn a_line_is_cleaned_of_escapes_prefixes_and_trailing_blanks() {
    let lines = excerpt_of(
        "\u{1b}[1;31mvm-test-run-example> \u{1b}[0mbefore\nmachine-1.2+x>   > !!! error: boom \t\r\n",
    );
    assert_eq!(lines, ["before", "error: boom"]);
}

#[test]
fn only_the_last_twenty_kept_lines_remain() {
    let mut text = String::new();
    for index in 1..=30 {
        writeln!(text, "error: number {index}").unwrap();
    }
    let lines = excerpt_of(&text);
    assert_eq!(lines.len(), 20);
    assert_eq!(lines[0], "error: number 11");
    assert_eq!(lines[19], "error: number 30");
}

struct Build {
    dir: TempDir,
}

struct Built {
    output: Output,
    calls: Vec<String>,
}

impl Built {
    fn stdout(&self) -> String {
        String::from_utf8(self.output.stdout.clone()).unwrap()
    }

    fn stderr(&self) -> String {
        String::from_utf8(self.output.stderr.clone()).unwrap()
    }
}

#[derive(Default)]
struct Fake<'a> {
    log: Option<PathBuf>,
    out_path: Option<&'a Path>,
    err_path: Option<&'a Path>,
    status: u8,
    env: Vec<(&'a str, &'a str)>,
}

impl Build {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).unwrap();
        common::write_executable(&bin.join("nix"), FAKE_NIX);
        symlink(common::find_program("cat"), bin.join("cat")).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn result(&self, file: &str) -> Value {
        serde_json::from_str(&fs::read_to_string(self.path(file)).unwrap()).unwrap()
    }

    fn run(&self, args: &[&str], fake: &Fake) -> Built {
        let calls = self.path("nix-calls");
        let _ = fs::remove_file(&calls);
        let mut command = common::sandboxed(
            Path::new(env!("CARGO_BIN_EXE_forge-ci")),
            self.dir.path(),
            &self.path("bin"),
            self.dir.path(),
        );
        command
            .arg("build")
            .args(args)
            .env("TMPDIR", self.dir.path())
            .env("NIX_CALLS", &calls)
            .env("STATUS", fake.status.to_string());
        if let Some(log) = &fake.log {
            command.env("LOG", log);
        }
        if let Some(path) = fake.out_path {
            command.env("OUT_PATH", path);
        }
        if let Some(path) = fake.err_path {
            command.env("ERR_PATH", path);
        }
        command.envs(fake.env.iter().copied());
        let output = command.output().unwrap();
        Built {
            output,
            calls: fs::read_to_string(&calls)
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect(),
        }
    }
}

#[test]
fn a_failing_vm_test_is_built_once_and_kept_failed_with_its_excerpt() {
    let runner = Build::new();

    let built = runner.run(
        &["x86_64-linux", "example-boots", "results"],
        &Fake {
            log: Some(log("timeout.log")),
            status: 1,
            ..Fake::default()
        },
    );

    assert_eq!(built.output.status.code(), Some(1));
    let result = runner.result("results/x86_64-linux-example-boots.json");
    assert_eq!(result["check"], "example-boots");
    assert_eq!(result["system"], "x86_64-linux");
    assert_eq!(result["outcome"], "failed");
    assert!(
        result["excerpt"].as_array().unwrap().iter().any(|line| line
            .as_str()
            .unwrap()
            .starts_with("RequestedAssertionFailed: action timed out")),
        "{result}"
    );
    assert_eq!(
        built.calls,
        [format!("{BUILD_LINE} .#checks.x86_64-linux.example-boots")]
    );
    assert!(
        built.stdout().contains("action timed out"),
        "the full log is not on stdout"
    );
}

#[test]
fn a_failing_build_has_no_evidence_and_the_excerpt_of_its_error() {
    let runner = Build::new();

    let built = runner.run(
        &["x86_64-linux", "example-toplevel", "results"],
        &Fake {
            log: Some(log("eval-error.log")),
            status: 1,
            ..Fake::default()
        },
    );

    assert_eq!(built.output.status.code(), Some(1));
    assert_eq!(
        runner.result("results/x86_64-linux-example-toplevel.json"),
        json!({
            "check": "example-toplevel",
            "system": "x86_64-linux",
            "outcome": "failed",
            "excerpt": ["error: undefined variable 'foo'"],
            "bootTime": null,
            "screenshots": []
        })
    );
}

#[test]
fn a_passing_check_keeps_the_evidence_of_its_output_only() {
    let runner = Build::new();
    let output = runner.path("check-output");
    fs::create_dir(&output).unwrap();
    fs::write(
        output.join("boot-time.json"),
        r#"{ "total": 7.04, "userspace": 3.0, "kernel": 0.4 }"#,
    )
    .unwrap();
    fs::write(output.join("desktop.png"), "png\n").unwrap();
    fs::write(output.join("perf.json"), r#"{ "rounds": 3, "hosts": {} }"#).unwrap();
    let logged = runner.path("log-output");
    fs::create_dir(&logged).unwrap();
    fs::write(
        logged.join("boot-time.json"),
        r#"{ "total": 99, "userspace": 99 }"#,
    )
    .unwrap();

    let built = runner.run(
        &["x86_64-linux", "app-hyprland", "evidence"],
        &Fake {
            out_path: Some(&output),
            err_path: Some(&logged),
            ..Fake::default()
        },
    );

    assert!(built.output.status.success(), "{}", built.stderr());
    let result = runner.result("evidence/x86_64-linux-app-hyprland.json");
    assert_eq!(result["outcome"], "passed");
    assert_eq!(result["excerpt"], json!([]));
    assert_eq!(result["bootTime"], json!({"total": 7.04, "userspace": 3.0}));
    assert_eq!(result["screenshots"], json!(["desktop.png"]));
    assert_eq!(
        fs::read_to_string(
            runner.path("evidence/screenshots/x86_64-linux-app-hyprland-desktop.png")
        )
        .unwrap(),
        "png\n"
    );
    assert_eq!(
        runner.result("evidence/perf/x86_64-linux.json")["rounds"],
        3
    );
}

#[test]
fn evidence_that_cannot_be_read_keeps_the_check_green() {
    let runner = Build::new();
    let output = runner.path("unreadable-output");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("boot-time.json"), "not json\n").unwrap();

    let built = runner.run(
        &["x86_64-linux", "example-boots", "unreadable"],
        &Fake {
            out_path: Some(&output),
            ..Fake::default()
        },
    );

    assert!(built.output.status.success(), "{}", built.stderr());
    let result = runner.result("unreadable/x86_64-linux-example-boots.json");
    assert_eq!(result["outcome"], "passed");
    assert_eq!(result["bootTime"], Value::Null);
    assert_eq!(result["screenshots"], json!([]));
    assert_eq!(
        built.stderr(),
        "ci-build: the boot time, measures or screenshots of example-boots could not be kept, the check still passed\n"
    );
}

fn migration(runner: &Build, extra: &[&str], env: Vec<(&str, &str)>) -> Built {
    let mut args = vec!["x86_64-linux", "feature-migration", "results"];
    args.extend(extra);
    let built = runner.run(
        &args,
        &Fake {
            env,
            ..Fake::default()
        },
    );
    assert!(built.output.status.success(), "{}", built.stderr());
    built
}

const AGAINST_THE_BASE: &str = "--override-input migration-base github:owner/repo/0123abc .#checks.x86_64-linux.feature-migration";
const AGAINST_THE_LOCK: &str = ".#checks.x86_64-linux.feature-migration";

#[test]
fn a_check_with_a_base_runs_against_it() {
    let runner = Build::new();

    let built = migration(
        &runner,
        &[
            "--base-input",
            "migration-base",
            "--base",
            "github:owner/repo/0123abc",
        ],
        Vec::new(),
    );

    assert_eq!(built.calls, [format!("{BUILD_LINE} {AGAINST_THE_BASE}")]);
    assert!(
        built.stdout().starts_with(
            "feature-migration runs against github:owner/repo/0123abc as migration-base\n"
        ),
        "{}",
        built.stdout()
    );
}

#[test]
fn an_all_zero_base_revision_is_left_to_the_locked_input() {
    let runner = Build::new();
    let base = "github:owner/repo/0000000000000000000000000000000000000000";

    let built = migration(
        &runner,
        &["--base-input", "migration-base", "--base", base],
        Vec::new(),
    );

    assert_eq!(built.calls, [format!("{BUILD_LINE} {AGAINST_THE_LOCK}")]);
    assert!(
        built.stdout().starts_with(&format!(
            "feature-migration has no base revision ({base}), it runs against its locked migration-base\n"
        )),
        "{}",
        built.stdout()
    );
}

#[test]
fn base_input_and_base_sha_stand_for_the_base_arguments() {
    let runner = Build::new();

    let built = migration(
        &runner,
        &[],
        vec![
            ("BASE_INPUT", "migration-base"),
            ("BASE_SHA", "0123abc"),
            ("GITHUB_REPOSITORY", "owner/repo"),
        ],
    );

    assert_eq!(built.calls, [format!("{BUILD_LINE} {AGAINST_THE_BASE}")]);
}

#[test]
fn an_empty_base_input_overrides_nothing() {
    let runner = Build::new();

    let built = migration(
        &runner,
        &[],
        vec![
            ("BASE_INPUT", ""),
            ("BASE_SHA", "0123abc"),
            ("GITHUB_REPOSITORY", "owner/repo"),
        ],
    );

    assert_eq!(built.calls, [format!("{BUILD_LINE} {AGAINST_THE_LOCK}")]);
}

const USAGE: &str = "usage: ci-build <system> <check> <out-dir> [--base-input <input> --base <flake-ref>]
  BASE_INPUT with BASE_SHA stands for --base-input $BASE_INPUT --base github:$GITHUB_REPOSITORY/$BASE_SHA
";

#[test]
fn a_base_without_the_input_it_overrides_or_a_bad_argument_is_a_usage_error() {
    let runner = Build::new();
    for args in [
        vec![
            "x86_64-linux",
            "feature-migration",
            "results",
            "--base",
            "github:owner/repo/0123abc",
        ],
        vec![
            "x86_64-linux",
            "feature-migration",
            "results",
            "--base-input",
            "migration-base",
        ],
        vec!["x86_64-linux", "feature-migration", "results", "--base"],
        vec!["x86_64-linux", "feature-migration", "results", "--other"],
        vec!["x86_64-linux", "feature-migration"],
    ] {
        let built = runner.run(&args, &Fake::default());

        assert_eq!(built.output.status.code(), Some(2), "{args:?}");
        assert_eq!(built.stderr(), USAGE, "{args:?}");
        assert!(built.calls.is_empty(), "{args:?}");
    }
}
