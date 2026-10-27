#[path = "../../forge/tests/common/mod.rs"]
mod common;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

const FAKE_NIX: &str = r#"
fail_build() {
  echo "error: builder for '/nix/store/x-$check.drv' failed with exit code 1" >&2
  exit 1
}

if [ "$*" = "eval --json .#ci.flakyMatrix" ]; then
  echo "$*" >>"$STATE/calls"
  cat "$MATRIX_FILE"
  exit 0
fi

[ "$1" = build ] || {
  echo "unexpected nix call: $*" >&2
  exit 1
}

installable=${*: -1}
check=${installable##*.}
echo "$*" >>"$STATE/calls"

if [[ " $* " == *" --rebuild "* ]]; then
  if [ ! -e "$STATE/$check.realised" ]; then
    echo "$check" >>"$STATE/rebuilt-before-realise"
    echo "error: some outputs of '/nix/store/x-$check.drv' are not valid, so checking is not possible" >&2
    exit 1
  fi

  count=$(($(cat "$STATE/$check.rebuilds" 2>/dev/null || echo 0) + 1))
  echo "$count" >"$STATE/$check.rebuilds"

  case "$check" in
  flaky) [ "$count" -ne 3 ] || fail_build ;;
  nondeterministic)
    echo "error: derivation '/nix/store/x-$check.drv' may not be deterministic: output \"/nix/store/x-$check\" differs" >&2
    exit 1
    ;;
  esac
  exit 0
fi

case "$check" in
broken) fail_build ;;
late)
  if [ ! -e "$STATE/late.tried" ]; then
    touch "$STATE/late.tried"
    fail_build
  fi
  ;;
esac

touch "$STATE/$check.realised"
"#;

const FAKE_EVAL_JOBS: &str = r#"
echo "nix-eval-jobs $*" >>"$STATE/calls"
cat "$EVALUATED_FILE"
"#;

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(sandbox.bin()).unwrap();
        fs::create_dir_all(sandbox.state()).unwrap();
        fs::write(sandbox.calls(), "").unwrap();
        let shebang = format!("#!{}\n", common::find_program("bash").display());
        common::write_executable(&sandbox.bin().join("nix"), &(shebang.clone() + FAKE_NIX));
        common::write_executable(
            &sandbox.bin().join("nix-eval-jobs"),
            &(shebang + FAKE_EVAL_JOBS),
        );
        sandbox
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn bin(&self) -> PathBuf {
        self.root().join("bin")
    }

    fn state(&self) -> PathBuf {
        self.root().join("state")
    }

    fn calls(&self) -> PathBuf {
        self.state().join("calls")
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    fn origin(&self) -> PathBuf {
        self.root().join("origin.git")
    }

    fn forge_ci(&self, cwd: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
        let mut path = OsString::from(self.bin());
        path.push(":");
        path.push(env::var_os("PATH").unwrap_or_default());
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flaky");
        let mut command = common::sandboxed(
            Path::new(env!("CARGO_BIN_EXE_forge-ci")),
            cwd,
            &self.bin(),
            self.root(),
        );
        command
            .env("PATH", path)
            .env("STATE", self.state())
            .env("MATRIX_FILE", fixtures.join("matrix.json"))
            .env("EVALUATED_FILE", fixtures.join("evaluated.jsonl"))
            .args(args);
        for (name, value) in envs {
            command.env(name, value);
        }
        command.output().unwrap()
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn runs_of(results: &Path, check: &str) -> Vec<bool> {
    let result: Value = serde_json::from_str(
        &fs::read_to_string(results.join(format!("x86_64-linux-{check}.json"))).unwrap(),
    )
    .unwrap();
    result["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| run["passed"].as_bool().unwrap())
        .collect()
}

#[test]
fn a_check_is_realised_once_then_rebuilt_for_every_run() {
    let sandbox = Sandbox::new();
    let results = sandbox.root().join("results");

    for check in ["steady", "flaky", "nondeterministic", "late", "broken"] {
        let output = sandbox.forge_ci(
            sandbox.root(),
            &["flaky", "x86_64-linux", check, "5", "results"],
            &[],
        );
        assert!(output.status.success(), "{check}: {output:?}");
        let stdout = text(&output.stdout);
        assert!(stdout.starts_with(&format!(
            "ci-flaky: realising {check} once, not counted as a run\n"
        )));
        assert!(stdout.contains(&format!("ci-flaky: run 5 of 5 of {check} passed: ")));
    }

    assert_eq!(runs_of(&results, "steady"), [true; 5]);
    assert_eq!(runs_of(&results, "flaky"), [true, true, false, true, true]);
    assert_eq!(runs_of(&results, "nondeterministic"), [true; 5]);
    assert_eq!(runs_of(&results, "late"), [true; 5]);
    assert_eq!(runs_of(&results, "broken"), [false; 5]);

    let flaky: Value =
        serde_json::from_str(&fs::read_to_string(results.join("x86_64-linux-flaky.json")).unwrap())
            .unwrap();
    assert_eq!(flaky["check"], "flaky");
    assert_eq!(flaky["system"], "x86_64-linux");
    assert!(
        flaky["runs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|run| run["seconds"].is_u64())
    );

    assert!(!sandbox.state().join("rebuilt-before-realise").exists());
    let calls = fs::read_to_string(sandbox.calls()).unwrap();
    assert_eq!(calls.matches("--rebuild").count(), 19, "{calls}");
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.ends_with("checks.x86_64-linux.broken"))
            .count(),
        6,
        "{calls}"
    );
}

#[test]
fn every_flaky_command_prints_its_usage() {
    let sandbox = Sandbox::new();
    let root = sandbox.root();

    let zero = sandbox.forge_ci(
        root,
        &["flaky", "x86_64-linux", "steady", "0", "results"],
        &[],
    );
    assert_eq!(zero.status.code(), Some(2));
    assert_eq!(
        text(&zero.stderr),
        "usage: ci-flaky <system> <check> <runs> <out-dir>\n"
    );

    let help = sandbox.forge_ci(root, &["flaky", "--help"], &[]);
    assert_eq!(help.status.code(), Some(0));
    assert_eq!(
        text(&help.stdout),
        "usage: ci-flaky <system> <check> <runs> <out-dir>\n"
    );

    let plan = sandbox.forge_ci(root, &["flaky-plan", "extra"], &[]);
    assert_eq!(plan.status.code(), Some(2));
    assert_eq!(text(&plan.stderr), "usage: ci-flaky-plan\n");

    let report = sandbox.forge_ci(root, &["flaky-report", "results", "history"], &[]);
    assert_eq!(report.status.code(), Some(2));
    assert_eq!(
        text(&report.stderr),
        "usage: MATRIX=<all> PLANNED=<checks> ci-flaky-report <results-dir> <history-out-dir>\n"
    );
}

struct Flake {
    sandbox: Sandbox,
}

struct Plan {
    checks: String,
    all: String,
    stderr: String,
}

impl Plan {
    fn planned(&self) -> String {
        let checks: Vec<Value> = serde_json::from_str(&self.checks).unwrap();
        let mut names: Vec<&str> = checks
            .iter()
            .map(|job| job["check"].as_str().unwrap())
            .collect();
        names.sort_unstable();
        names.join(" ")
    }

    fn all_count(&self) -> usize {
        serde_json::from_str::<Vec<Value>>(&self.all).unwrap().len()
    }
}

struct Report {
    code: Option<i32>,
    markdown: String,
    history: PathBuf,
}

impl Report {
    fn history(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(&self.history).unwrap()).unwrap()
    }

    fn has_line(&self, line: &str) -> bool {
        self.markdown.lines().any(|candidate| candidate == line)
    }
}

impl Flake {
    fn new() -> Self {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo();
        let origin = sandbox.origin();
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&origin).unwrap();
        git(&origin, &["init", "-q", "--bare"]);
        git(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("flake.nix"), "source\n").unwrap();
        git(&repo, &["add", "flake.nix"]);
        git(&repo, &["commit", "-q", "-m", "main"]);
        git(
            &repo,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        git(&repo, &["push", "-q", "origin", "main"]);
        Self { sandbox }
    }

    fn tree(&self) -> String {
        git(&self.sandbox.repo(), &["rev-parse", "HEAD^{tree}"])
    }

    fn publish(&self, flaky: &str) {
        let work = tempfile::tempdir().unwrap();
        git(work.path(), &["init", "-q", "-b", "badges"]);
        fs::write(work.path().join("flaky.json"), flaky).unwrap();
        git(work.path(), &["add", "flaky.json"]);
        git(work.path(), &["commit", "-q", "-m", "badges"]);
        git(
            work.path(),
            &[
                "push",
                "-q",
                "-f",
                self.sandbox.origin().to_str().unwrap(),
                "badges",
            ],
        );
    }

    fn publish_file(&self, path: &Path) {
        self.publish(&fs::read_to_string(path).unwrap());
    }

    fn run_plan(&self) -> Output {
        fs::write(self.sandbox.calls(), "").unwrap();
        self.sandbox
            .forge_ci(&self.sandbox.repo(), &["flaky-plan"], &[])
    }

    fn plan(&self) -> Plan {
        let output = self.run_plan();
        assert!(output.status.success(), "{output:?}");
        let stdout = text(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 2, "{stdout}");
        Plan {
            checks: lines[0].strip_prefix("checks=").unwrap().to_owned(),
            all: lines[1].strip_prefix("all=").unwrap().to_owned(),
            stderr: text(&output.stderr),
        }
    }

    fn calls(&self) -> String {
        fs::read_to_string(self.sandbox.calls()).unwrap()
    }

    fn result(&self, night: &str, check: &str, runs: &Value) {
        let dir = self.sandbox.root().join(format!("{night}-results"));
        fs::create_dir_all(&dir).unwrap();
        let result = json!({ "check": check, "system": "x86_64-linux", "runs": runs });
        fs::write(
            dir.join(format!("x86_64-linux-{check}.json")),
            result.to_string(),
        )
        .unwrap();
    }

    fn report(&self, night: &str, plan: &Plan) -> Report {
        self.report_with(night, &plan.all, &plan.checks)
    }

    fn report_with(&self, night: &str, matrix: &str, planned: &str) -> Report {
        let results = self.sandbox.root().join(format!("{night}-results"));
        let history = self.sandbox.root().join(format!("{night}-history"));
        let output = self.sandbox.forge_ci(
            &self.sandbox.repo(),
            &[
                "flaky-report",
                results.to_str().unwrap(),
                history.to_str().unwrap(),
            ],
            &[("MATRIX", matrix), ("PLANNED", planned)],
        );
        Report {
            code: output.status.code(),
            markdown: text(&output.stdout) + &text(&output.stderr),
            history: history.join("flaky.json"),
        }
    }
}

fn record(drv: &str, check: &str, passed: &[bool]) -> (String, Value) {
    let runs: Vec<Value> = passed
        .iter()
        .map(|passed| json!({ "passed": passed, "seconds": 7 }))
        .collect();
    (
        format!("/nix/store/{drv}.drv"),
        json!({ "check": check, "system": "x86_64-linux", "runs": runs }),
    )
}

fn runs(seconds: &[u64], failed: Option<usize>) -> Value {
    seconds
        .iter()
        .enumerate()
        .map(|(index, seconds)| json!({ "passed": Some(index) != failed, "seconds": seconds }))
        .collect()
}

#[test]
fn without_a_badges_branch_every_vm_check_is_planned() {
    let flake = Flake::new();

    let plan = flake.plan();

    assert_eq!(plan.planned(), "changed flaky new steady");
    let checks: Vec<Value> = serde_json::from_str(&plan.checks).unwrap();
    assert!(
        checks
            .iter()
            .all(|job| job["runs"] == 5 && job["runner"] == "ubuntu-24.04")
    );
    assert_eq!(
        checks.iter().find(|job| job["check"] == "changed").unwrap()["drvPath"],
        "/nix/store/c2.drv"
    );
    assert_eq!(plan.all_count(), 4);
    assert!(plan.stderr.contains("origin has no badges branch yet"));
    assert!(plan.stderr.contains(
        "flaky-history: no valid flaky.json on the badges branch, every VM check is measured"
    ));
    assert!(
        flake
            .calls()
            .contains("nix-eval-jobs --flake .#checks --force-recurse --workers ")
    );
}

#[test]
fn an_origin_that_cannot_be_fetched_stops_the_plan_and_the_report() {
    let flake = Flake::new();
    let all = flake.plan().all;
    let unreachable = flake.sandbox.root().join("unreachable.git");
    git(
        &flake.sandbox.repo(),
        &["remote", "set-url", "origin", unreachable.to_str().unwrap()],
    );

    let output = flake.run_plan();
    assert_ne!(output.status.code(), Some(0));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("could not be fetched"), "{stderr}");
    assert!(stderr.contains(
        "flaky-history: without the record, every VM check would be measured again; fix the fetch instead"
    ));
    assert!(!stderr.contains("no valid flaky.json"), "{stderr}");
    assert!(text(&output.stdout).is_empty());

    let report = flake.report_with("unreachable", &all, "[]");
    assert_eq!(report.code, Some(1), "{}", report.markdown);
    assert!(report.markdown.contains("could not be fetched"));
    assert!(!report.history.exists());
}

#[test]
fn a_malformed_or_misshapen_record_plans_every_check() {
    let flake = Flake::new();
    let (key, steady) = record("s1", "steady", &[true; 5]);

    for flaky in [
        "not json".to_owned(),
        json!({ "tree": 3, "checks": { key.clone(): steady.clone() } }).to_string(),
    ] {
        flake.publish(&flaky);

        let plan = flake.plan();

        assert_eq!(plan.planned(), "changed flaky new steady", "{flaky}");
        assert!(plan.stderr.contains("no valid flaky.json"), "{flaky}");
    }
}

#[test]
fn each_night_measures_only_what_changed_or_failed_and_records_a_green_tree() {
    let flake = Flake::new();
    let checks: serde_json::Map<String, Value> = [
        record("s1", "steady", &[true; 5]),
        record("f1", "flaky", &[true, true, false, true, true]),
        record("c1", "changed", &[true; 5]),
    ]
    .into_iter()
    .collect();
    flake.publish(&json!({ "tree": null, "checks": checks }).to_string());

    let night1 = flake.plan();
    assert_eq!(night1.planned(), "changed flaky new");

    flake.result("night1", "flaky", &runs(&[5, 1, 4, 2, 3], Some(2)));
    flake.result("night1", "changed", &runs(&[10, 60, 20, 50, 30, 40], None));
    let report = flake.report("night1", &night1);
    assert_ne!(report.code, Some(0), "{}", report.markdown);
    for line in [
        "## Flaky VM checks",
        "Each VM check is realised once, then rebuilt with `nix build --rebuild` for every run. A run that fails is a test that does not always pass on the same code. A check whose derivation already passed all of its runs is not rerun.",
        "| Check | System | Pass rate | Median time |",
        "|---|---|---|---|",
        "| `steady` | x86_64-linux | 100 % (not rerun, unchanged) | 7 s |",
        "| `flaky` | x86_64-linux | 80 % | 3 s |",
        "| `changed` | x86_64-linux | 100 % | 35 s |",
        "| `new` | x86_64-linux | no result | |",
        "`flaky` passed 4 of its 5 runs.",
        "`new` wrote no result: its job failed or hit its time limit, see its log.",
    ] {
        assert!(
            report.has_line(line),
            "{line} is missing from {}",
            report.markdown
        );
    }
    assert!(!report.markdown.contains("Every VM check passed"));
    let history = report.history();
    let keys: Vec<&String> = history["checks"].as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        [
            "/nix/store/c2.drv",
            "/nix/store/f1.drv",
            "/nix/store/s1.drv"
        ]
    );
    assert_eq!(history["tree"], Value::Null);

    flake.publish_file(&report.history);
    let night2 = flake.plan();
    assert_eq!(night2.planned(), "flaky new");

    flake.result("night2", "flaky", &runs(&[2; 5], None));
    flake.result("night2", "new", &runs(&[9; 5], None));
    let report = flake.report("night2", &night2);
    assert_eq!(report.code, Some(0), "{}", report.markdown);
    assert!(report.has_line("Every VM check passed all of its runs."));
    assert_eq!(report.history()["tree"], flake.tree());

    flake.publish_file(&report.history);
    let quiet = flake.plan();
    assert_eq!((quiet.checks.as_str(), quiet.all.as_str()), ("[]", "[]"));
    assert!(quiet.stderr.contains(
        "ci-flaky-plan: this tree was already measured and every VM check passed all of its runs"
    ));
    assert!(!flake.calls().contains("eval"), "{}", flake.calls());

    git(
        &flake.sandbox.repo(),
        &["commit", "-q", "--allow-empty", "-m", "same tree"],
    );
    assert_eq!(flake.plan().all, "[]");

    fs::write(flake.sandbox.repo().join("README.md"), "docs\n").unwrap();
    git(&flake.sandbox.repo(), &["add", "README.md"]);
    git(&flake.sandbox.repo(), &["commit", "-q", "-m", "docs"]);
    let docs = flake.plan();
    assert!(flake.calls().contains("eval"));
    assert_eq!(docs.checks, "[]");
    assert_eq!(docs.all_count(), 4);
    let report = flake.report("docs", &docs);
    assert_eq!(report.code, Some(0), "{}", report.markdown);
    assert_eq!(
        report.markdown.matches("(not rerun, unchanged)").count(),
        4,
        "{}",
        report.markdown
    );
    assert_eq!(report.history()["tree"], flake.tree());
}
