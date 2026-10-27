mod common;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::{Value, json};
use tempfile::TempDir;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/plan");

const FAKE_NIX: &str = r#"#!/bin/sh
[ "$1 $2" = "eval --json" ] || exit 1
name=${3#.#ci.}
if [ "$name" = matrix ]; then
  exec cat "$MATRIX"
fi
exec cat "$PLAN/$name.json"
"#;

const FAKE_EVAL_JOBS: &str = "#!/bin/sh\nexec cat \"$EVALUATED\"\n";

struct Plan {
    dir: TempDir,
}

struct Planned {
    output: Output,
    summary: String,
}

impl Planned {
    fn stdout(&self) -> String {
        String::from_utf8(self.output.stdout.clone()).unwrap()
    }

    fn stderr(&self) -> String {
        String::from_utf8(self.output.stderr.clone()).unwrap()
    }

    fn value(&self, key: &str) -> Value {
        let prefix = format!("{key}=");
        let stdout = self.stdout();
        let line = stdout
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .unwrap_or_else(|| panic!("{key} is missing from {stdout}"));
        serde_json::from_str(line).unwrap()
    }

    fn names(&self, key: &str) -> Vec<String> {
        self.value(key)
            .as_array()
            .unwrap()
            .iter()
            .map(|job| job["check"].as_str().unwrap().to_owned())
            .collect()
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(FIXTURES).join(name)
}

fn fixture_json(name: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(fixture(name)).unwrap()).unwrap()
}

impl Plan {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).unwrap();
        common::write_executable(&bin.join("nix"), FAKE_NIX);
        common::write_executable(&bin.join("nix-eval-jobs"), FAKE_EVAL_JOBS);
        symlink(common::find_program("cat"), bin.join("cat")).unwrap();
        Self { dir }
    }

    fn run(&self, args: &[&str], matrix: &str, evaluated: &str) -> Planned {
        let summary = self.dir.path().join("summary.md");
        let _ = fs::remove_file(&summary);
        let output = common::sandboxed(
            Path::new(env!("CARGO_BIN_EXE_forge-ci")),
            self.dir.path(),
            &self.dir.path().join("bin"),
            self.dir.path(),
        )
        .arg("plan")
        .args(args)
        .env("PLAN", FIXTURES)
        .env("MATRIX", fixture(matrix))
        .env("EVALUATED", fixture(evaluated))
        .env("GITHUB_STEP_SUMMARY", &summary)
        .output()
        .unwrap();
        Planned {
            output,
            summary: fs::read_to_string(&summary).unwrap_or_default(),
        }
    }

    fn plan(&self, event: &str, matrix: &str, evaluated: &str) -> Planned {
        let planned = self.run(&[event], matrix, evaluated);
        assert!(
            planned.output.status.success(),
            "{event}: ci-plan exits with {:?}: {}",
            planned.output.status.code(),
            planned.stderr()
        );
        planned
    }
}

#[test]
fn a_pull_request_skips_the_cached_checks_and_the_ones_compiling_a_kernel() {
    let planned = Plan::new().plan("pull_request", "matrix.json", "evaluated.jsonl");

    assert_eq!(
        planned.names("checks"),
        ["example-boots", "feature-migration", "example-pi-sd-image"]
    );
    assert_eq!(
        planned.value("skipped"),
        json!([
            {"check":"example-toplevel","system":"x86_64-linux","vm":false,"reason":"compiles a kernel"},
            {"check":"docs","system":"x86_64-linux","vm":false,"reason":"cached"}
        ])
    );
    let checks = planned.value("checks");
    assert_eq!(
        checks[0],
        json!({"check":"example-boots","system":"x86_64-linux","runner":"ubuntu-24.04","vm":true,"baseInput":null,"timeout":60})
    );
    assert_eq!(checks[1]["baseInput"], "migration-base");
    assert_eq!(
        planned.value("closures"),
        json!([
            {"system":"x86_64-linux","runner":"ubuntu-24.04","build":false},
            {"system":"aarch64-linux","runner":"ubuntu-24.04-arm","build":true}
        ])
    );
    assert_eq!(planned.value("perf"), fixture_json("perfMatrix.json"));
    assert_eq!(planned.value("hostChecks"), fixture_json("hostChecks.json"));
    assert_eq!(
        planned.value("descriptions"),
        fixture_json("descriptions.json")
    );
    assert!(
        planned
            .summary
            .lines()
            .any(|line| line == "3 checks to build."),
        "{}",
        planned.summary
    );
    assert!(
        planned
            .summary
            .lines()
            .any(|line| line == "- `example-toplevel` (x86_64-linux): compiles a kernel"),
        "{}",
        planned.summary
    );
}

#[test]
fn every_output_is_one_github_output_line_of_compact_json() {
    let planned = Plan::new().plan("pull_request", "matrix.json", "evaluated.jsonl");

    let stdout = planned.stdout();
    let keys: Vec<&str> = stdout
        .lines()
        .map(|line| {
            let (key, value) = line.split_once('=').unwrap();
            serde_json::from_str::<Value>(value).unwrap();
            assert!(
                !value.contains(": ") && !value.contains(", "),
                "not compact: {line}"
            );
            key
        })
        .collect();
    assert_eq!(
        keys,
        [
            "checks",
            "skipped",
            "closures",
            "perf",
            "hostChecks",
            "descriptions"
        ]
    );
    assert!(stdout.ends_with('\n'));
}

#[test]
fn the_summary_lists_the_count_then_every_skipped_check() {
    let planned = Plan::new().plan("pull_request", "matrix.json", "evaluated.jsonl");

    assert_eq!(
        planned.summary,
        "### Plan\n\n3 checks to build.\n- `example-toplevel` (x86_64-linux): compiles a kernel\n- `docs` (x86_64-linux): cached\n"
    );
}

#[test]
fn a_push_builds_the_checks_compiling_a_kernel_with_a_longer_timeout() {
    let planned = Plan::new().plan("push", "matrix.json", "evaluated.jsonl");

    assert_eq!(
        planned.names("checks"),
        [
            "example-toplevel",
            "example-boots",
            "feature-migration",
            "example-pi-sd-image"
        ]
    );
    assert_eq!(planned.value("checks")[0]["timeout"], 240);
    assert_eq!(
        planned.value("skipped"),
        json!([{"check":"docs","system":"x86_64-linux","vm":false,"reason":"cached"}])
    );
}

#[test]
fn a_fully_cached_plan_builds_nothing_and_every_closure() {
    let plan = Plan::new();
    for event in ["push", "pull_request"] {
        let planned = plan.plan(event, "matrix-without-base.json", "cached.jsonl");

        assert!(
            planned.stdout().lines().any(|line| line == "checks=[]"),
            "{event}: {}",
            planned.stdout()
        );
        assert_eq!(
            planned.value("skipped"),
            json!([
                {"check":"example-toplevel","system":"x86_64-linux","vm":false,"reason":"cached"},
                {"check":"example-pi-sd-image","system":"aarch64-linux","vm":false,"reason":"cached"}
            ])
        );
        assert!(
            planned
                .value("closures")
                .as_array()
                .unwrap()
                .iter()
                .all(|closure| closure["build"] == true)
        );
        assert!(
            planned
                .summary
                .lines()
                .any(|line| line == "0 checks to build."),
            "{event}: {}",
            planned.summary
        );
    }
}

#[test]
fn a_check_that_does_not_evaluate_fails_the_plan_and_is_named() {
    let planned = Plan::new().run(&["push"], "matrix.json", "broken.jsonl");

    assert_eq!(planned.output.status.code(), Some(1));
    assert!(
        planned.stderr().contains(
            "x86_64-linux.example-boots does not evaluate: error: undefined variable 'foo'"
        ),
        "{}",
        planned.stderr()
    );
    assert!(
        !planned
            .stdout()
            .lines()
            .any(|line| line.starts_with("checks=")),
        "{}",
        planned.stdout()
    );
    assert_eq!(planned.summary, "");
}

#[test]
fn a_check_missing_from_the_evaluation_fails_the_plan_and_is_named() {
    let planned = Plan::new().run(&["push"], "matrix.json", "missing.jsonl");

    assert_eq!(planned.output.status.code(), Some(1));
    assert!(
        planned
            .stderr()
            .contains("x86_64-linux.example-boots is missing from the evaluation"),
        "{}",
        planned.stderr()
    );
    assert_eq!(planned.stdout(), "");
}

#[test]
fn a_plan_without_its_event_is_a_usage_error() {
    let planned = Plan::new().run(&[], "matrix.json", "evaluated.jsonl");

    assert_eq!(planned.output.status.code(), Some(2));
    assert_eq!(planned.stderr(), "usage: ci-plan <event>\n");
}
