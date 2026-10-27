mod common;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use forge::system::Machine;
use forge_ci::badges::flaky_history;
use forge_ci::model::FlakyHistory;
use serde_json::{Value, json};
use tempfile::TempDir;

const USAGE: &str = "usage: ci-badges checks <out-dir>
       ci-badges reproducibility <summary.md> <out-dir>
       ci-badges publish <dir>
       ci-badges read <file>
";

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/runs")
            .join(name),
    )
    .unwrap()
}

struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        symlink(common::find_program("git"), bin.join("git")).unwrap();
        fs::create_dir_all(root.path().join("home")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    fn command(&self, dir: &str) -> Command {
        common::sandboxed(
            Path::new(env!("CARGO_BIN_EXE_forge-ci")),
            &self.path(dir),
            &self.path("bin"),
            &self.path("home"),
        )
    }

    fn badges(&self, dir: &str, args: &[&str]) -> Output {
        self.command(dir).arg("badges").args(args).output().unwrap()
    }

    fn checks(&self, dir: &str, env: &[(&str, &str)]) -> Output {
        let mut command = self.command(".");
        command
            .env("RESULTS_DIR", dir)
            .env("HOST_CHECKS", fixture("host-checks.json"))
            .env("PLANNED", fixture("planned.json"))
            .env("SKIPPED", "[]")
            .env("LINT_RESULT", "success");
        for (name, value) in env {
            command.env(name, value);
        }
        let output = command
            .args(["badges", "checks", &format!("{dir}/badges")])
            .output()
            .unwrap();
        assert!(output.status.success(), "{dir}: {output:?}");
        output
    }

    fn pass_all(&self, dir: &str) {
        let results = self.path(dir);
        fs::create_dir_all(&results).unwrap();
        let planned: Vec<Value> = serde_json::from_str(&fixture("planned.json")).unwrap();
        for job in planned {
            let (check, system) = (
                job["check"].as_str().unwrap(),
                job["system"].as_str().unwrap(),
            );
            let result = json!({ "check": check, "system": system, "outcome": "passed", "excerpt": [], "bootTime": null, "screenshots": [] });
            fs::write(
                results.join(format!("{system}-{check}.json")),
                result.to_string(),
            )
            .unwrap();
        }
    }

    fn set_field(&self, file: &str, field: &str, value: Value) {
        let path = self.path(file);
        let mut result: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        result[field] = value;
        fs::write(path, result.to_string()).unwrap();
    }

    fn git(&self, dir: &str, args: &[&str]) {
        let output = self.git_out(
            dir,
            &[&["-c", "user.name=t", "-c", "user.email=t@t"], args].concat(),
        );
        assert!(output.status.success(), "git {args:?}: {output:?}");
    }

    fn git_out(&self, dir: &str, args: &[&str]) -> Output {
        let mut command = Command::new(common::find_program("git"));
        command
            .current_dir(self.path(dir))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", self.path("home"))
            .args(args);
        command.output().unwrap()
    }

    fn repo(&self) {
        fs::create_dir_all(self.path("origin.git")).unwrap();
        fs::create_dir_all(self.path("repo")).unwrap();
        let git = |dir: &str, args: &[&str]| self.git(dir, args);
        git("origin.git", &["init", "-q", "--bare"]);
        git("repo", &["init", "-q", "-b", "main"]);
        git("repo", &["commit", "-q", "--allow-empty", "-m", "main"]);
        fs::write(self.path("repo/flake.nix"), "source\n").unwrap();
        git("repo", &["add", "flake.nix"]);
        git("repo", &["commit", "-q", "-m", "source"]);
        let origin = self.path("origin.git");
        git(
            "repo",
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        git("repo", &["push", "-q", "origin", "main"]);
    }

    fn origin(&self, args: &[&str]) -> String {
        let output = self.git_out("origin.git", args);
        String::from_utf8(output.stdout).unwrap()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[track_caller]
fn expect_badge(file: &Path, message: &str, color: &str) {
    let text =
        fs::read_to_string(file).unwrap_or_else(|_| panic!("{} was not written", file.display()));
    let badge: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(badge["schemaVersion"], 1, "{text}");
    assert!(badge["label"].is_string(), "{text}");
    assert_eq!(badge["message"], message, "{text}");
    assert_eq!(badge["color"], color, "{text}");
}

#[test]
fn a_green_run_counts_every_check_vm_test_host_and_the_boot_time() {
    let sandbox = Sandbox::new();
    sandbox.pass_all("green");
    sandbox.set_field(
        "green/x86_64-linux-example-boots.json",
        "bootTime",
        json!({ "total": 7.04, "userspace": 3.0 }),
    );
    sandbox.set_field(
        "green/x86_64-linux-example-server-boots.json",
        "bootTime",
        json!({ "total": 12, "userspace": 5 }),
    );

    let output = sandbox.checks("green", &[]);

    let out = sandbox.path("green/badges");
    expect_badge(&out.join("checks.json"), "12/12 passing", "brightgreen");
    expect_badge(&out.join("vm-tests.json"), "7", "5277C3");
    expect_badge(&out.join("hosts.json"), "3 built", "5277C3");
    expect_badge(&out.join("boot-time.json"), "7.0 s", "5277C3");
    assert_eq!(
        fs::read_to_string(out.join("checks.json")).unwrap(),
        "{\n  \"schemaVersion\": 1,\n  \"label\": \"checks\",\n  \"message\": \"12/12 passing\",\n  \"color\": \"brightgreen\"\n}\n"
    );
    assert_eq!(
        fs::read_to_string(out.join("boot-time.json")).unwrap(),
        "{\n  \"schemaVersion\": 1,\n  \"label\": \"boot to multi-user\",\n  \"message\": \"7.0 s\",\n  \"color\": \"5277C3\"\n}\n"
    );
    assert_eq!(
        stdout(&output),
        "boot-time.json\nchecks.json\nhosts.json\nvm-tests.json\n"
    );
}

#[test]
fn cached_checks_pass_and_the_boot_time_comes_from_the_current_measure() {
    let sandbox = Sandbox::new();
    sandbox.pass_all("cached");
    fs::remove_file(sandbox.path("cached/x86_64-linux-example-boots.json")).unwrap();
    fs::remove_file(sandbox.path("cached/x86_64-linux-feature-ssh.json")).unwrap();
    fs::create_dir_all(sandbox.path("cached/current")).unwrap();
    fs::write(
        sandbox.path("cached/current/boot-time.json"),
        r#"{ "example": { "total": 6.84, "userspace": 2.1 } }"#,
    )
    .unwrap();
    let planned: Vec<Value> = serde_json::from_str(&fixture("planned.json")).unwrap();
    let planned: Vec<Value> = planned
        .into_iter()
        .filter(|job| job["check"] != "example-boots" && job["check"] != "feature-ssh")
        .collect();

    sandbox.checks(
        "cached",
        &[
            ("PLANNED", &Value::from(planned).to_string()),
            (
                "SKIPPED",
                r#"[{"check":"example-boots","system":"x86_64-linux","vm":true,"reason":"cached"},{"check":"feature-ssh","system":"x86_64-linux","vm":true,"reason":"cached"}]"#,
            ),
        ],
    );

    let out = sandbox.path("cached/badges");
    expect_badge(&out.join("checks.json"), "12/12 passing", "brightgreen");
    expect_badge(&out.join("vm-tests.json"), "7", "5277C3");
    expect_badge(&out.join("boot-time.json"), "6.8 s", "5277C3");
}

#[test]
fn a_red_run_counts_the_failing_checks_and_the_hosts_not_built() {
    let sandbox = Sandbox::new();
    sandbox.pass_all("red");
    sandbox.set_field(
        "red/x86_64-linux-example-server-toplevel.json",
        "outcome",
        json!("failed"),
    );
    fs::remove_file(sandbox.path("red/x86_64-linux-example-disk.json")).unwrap();

    sandbox.checks("red", &[("LINT_RESULT", "failure")]);

    let out = sandbox.path("red/badges");
    expect_badge(&out.join("checks.json"), "3/12 failing", "red");
    expect_badge(&out.join("hosts.json"), "2/3 built", "red");
    assert!(
        !out.join("boot-time.json").exists(),
        "a boot time is published without one recorded"
    );
}

#[test]
fn a_flake_that_does_not_evaluate_publishes_only_the_checks_badge() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.path("no-plan")).unwrap();
    fs::write(sandbox.path("no-plan/broken.json"), "{").unwrap();

    let output = sandbox.checks(
        "no-plan",
        &[
            ("PLANNED", ""),
            ("SKIPPED", ""),
            ("HOST_CHECKS", ""),
            ("LINT_RESULT", "failure"),
        ],
    );

    let out = sandbox.path("no-plan/badges");
    expect_badge(&out.join("checks.json"), "flake does not evaluate", "red");
    assert_eq!(stdout(&output), "checks.json\n");
}

fn reproducibility(sandbox: &Sandbox, summary: &str, out: &str) -> Output {
    fs::write(sandbox.path(&format!("{out}.md")), summary).unwrap();
    let output = sandbox.badges(".", &["reproducibility", &format!("{out}.md"), out]);
    assert!(output.status.success(), "{out}: {output:?}");
    output
}

#[test]
fn the_reproducibility_badge_reads_the_summary() {
    let sandbox = Sandbox::new();

    let identical = reproducibility(
        &sandbox,
        "## Reproducibility\n\n- `example`: the rebuild is identical.\n- `example-server`: the rebuild is identical.\n",
        "identical",
    );
    expect_badge(
        &sandbox.path("identical/reproducibility.json"),
        "reproducible",
        "brightgreen",
    );
    assert_eq!(
        stdout(&identical),
        fs::read_to_string(sandbox.path("identical/reproducibility.json")).unwrap()
    );

    reproducibility(
        &sandbox,
        "- `example`: the rebuild differs from the first build.\n\n<details>\n<summary>diffoscope of example, first 200 lines</summary>\n</details>\n\n- `example-server`: the rebuild differs from the first build.\n- `example-desktop`: the rebuild is identical.\n",
        "differs",
    );
    expect_badge(
        &sandbox.path("differs/reproducibility.json"),
        "2 paths differ",
        "orange",
    );

    reproducibility(
        &sandbox,
        "- `example`: the rebuild differs from the first build.\n",
        "one",
    );
    expect_badge(
        &sandbox.path("one/reproducibility.json"),
        "1 path differs",
        "orange",
    );

    reproducibility(
        &sandbox,
        "- `example`: the rebuild is identical.\n- `example-server`: the system did not build, see the job log.\n",
        "broken",
    );
    expect_badge(
        &sandbox.path("broken/reproducibility.json"),
        "not finished",
        "lightgrey",
    );

    reproducibility(&sandbox, "## Reproducibility\n", "empty");
    expect_badge(
        &sandbox.path("empty/reproducibility.json"),
        "no host",
        "lightgrey",
    );
}

#[test]
fn a_missing_summary_fails() {
    let sandbox = Sandbox::new();
    let output = sandbox.badges(".", &["reproducibility", "missing.md", "out"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "ci-badges: missing.md does not exist\n");
}

#[test]
fn wrong_arguments_print_the_usage_and_exit_2() {
    let sandbox = Sandbox::new();
    for args in [
        &[][..],
        &["checks"][..],
        &["reproducibility", "summary.md"][..],
        &["publish", "a", "b"][..],
        &["read"][..],
        &["other", "x"][..],
    ] {
        let output = sandbox.badges(".", args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(stderr(&output), USAGE, "{args:?}");
    }

    let help = sandbox.badges(".", &["--help"]);
    assert!(help.status.success());
    assert_eq!(stdout(&help), USAGE);
}

#[test]
fn publish_keeps_an_orphan_branch_of_every_badge_and_read_gives_them_back() {
    let sandbox = Sandbox::new();
    sandbox.repo();
    fs::create_dir_all(sandbox.path("green/badges")).unwrap();
    for name in ["boot-time", "checks", "hosts", "vm-tests"] {
        fs::write(
            sandbox.path(&format!("green/badges/{name}.json")),
            format!("{{\"label\":\"{name}\"}}\n"),
        )
        .unwrap();
    }
    fs::create_dir_all(sandbox.path("repro")).unwrap();
    fs::write(sandbox.path("repro/reproducibility.json"), "{}\n").unwrap();

    let first = sandbox.badges("repo", &["publish", "../green/badges"]);
    assert!(first.status.success(), "{first:?}");
    assert_eq!(
        stdout(&first),
        "ci-badges: published boot-time.json checks.json hosts.json vm-tests.json\n"
    );
    let second = sandbox.badges("repo", &["publish", "../repro"]);
    assert!(second.status.success(), "{second:?}");

    assert_eq!(
        sandbox.origin(&["ls-tree", "--name-only", "badges"]),
        "boot-time.json\nchecks.json\nhosts.json\nreproducibility.json\nvm-tests.json\n"
    );
    assert!(
        !sandbox
            .git_out("origin.git", &["merge-base", "badges", "main"])
            .status
            .success(),
        "the badges branch shares history with main"
    );
    assert_eq!(sandbox.origin(&["rev-list", "--count", "badges"]), "2\n");

    let unchanged = sandbox.badges("repo", &["publish", "../repro"]);
    assert!(unchanged.status.success(), "{unchanged:?}");
    assert_eq!(stdout(&unchanged), "ci-badges: the badges did not change\n");
    assert_eq!(sandbox.origin(&["rev-list", "--count", "badges"]), "2\n");
    let worktrees = sandbox.git_out("repo", &["worktree", "list"]);
    assert_eq!(
        String::from_utf8(worktrees.stdout).unwrap().lines().count(),
        1,
        "a worktree is left behind"
    );

    let read = sandbox.badges("repo", &["read", "checks.json"]);
    assert!(read.status.success(), "{read:?}");
    assert_eq!(stdout(&read), "{\"label\":\"checks\"}\n");
    let missing = sandbox.badges("repo", &["read", "missing.json"]);
    assert_eq!(missing.status.code(), Some(3));
    assert_eq!(
        stderr(&missing),
        "ci-badges: the badges branch has no missing.json\n"
    );

    let unreachable = sandbox.path("unreachable.git");
    sandbox.git(
        "repo",
        &["remote", "set-url", "origin", unreachable.to_str().unwrap()],
    );
    let failed = sandbox.badges("repo", &["read", "checks.json"]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(
        stderr(&failed).contains("could not be fetched"),
        "{failed:?}"
    );
    let failed = sandbox.badges("repo", &["publish", "../green/badges"]);
    assert_eq!(failed.status.code(), Some(1), "{failed:?}");
    assert_eq!(
        stderr(&failed),
        "ci-badges: the badges branch of origin could not be fetched\n"
    );
}

#[test]
fn reading_a_repository_without_a_badges_branch_exits_3() {
    let sandbox = Sandbox::new();
    sandbox.repo();
    let output = sandbox.badges("repo", &["read", "checks.json"]);
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        stderr(&output),
        "ci-badges: origin has no badges branch yet\n"
    );
}

fn history_of(sandbox: &Sandbox) -> (Result<FlakyHistory, String>, String) {
    let mut err = Vec::new();
    let repo = sandbox.path("repo");
    let history = flaky_history(&Machine, repo.to_str().unwrap(), &mut err)
        .map_err(|error| error.to_string());
    (history, String::from_utf8(err).unwrap())
}

fn publish_flaky(sandbox: &Sandbox, text: &str) {
    let dir = sandbox.path("history");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("flaky.json"), text).unwrap();
    let output = sandbox.badges("repo", &["publish", "../history"]);
    assert!(output.status.success(), "{output:?}");
}

const DEGRADED: &str =
    "flaky-history: no valid flaky.json on the badges branch, every VM check is measured\n";

#[test]
fn the_flaky_history_is_read_from_the_badges_branch() {
    let sandbox = Sandbox::new();
    sandbox.repo();

    let (history, err) = history_of(&sandbox);
    assert_eq!(history, Ok(FlakyHistory::default()));
    assert_eq!(
        err,
        format!("ci-badges: origin has no badges branch yet\n{DEGRADED}")
    );

    publish_flaky(
        &sandbox,
        r#"{"tree":"abc","checks":{"/nix/store/a.drv":{"check":"a","system":"x86_64-linux","runs":[{"passed":true,"seconds":12}]}}}"#,
    );
    let (history, err) = history_of(&sandbox);
    let history = history.unwrap();
    assert_eq!(history.tree.as_deref(), Some("abc"));
    assert_eq!(history.checks["/nix/store/a.drv"].runs.len(), 1);
    assert_eq!(err, "");

    for invalid in [
        "null",
        "{\"tree\":null,\"checks\":{}}\n{\"tree\":null,\"checks\":{}}",
        r#"{"tree":1,"checks":{}}"#,
        r#"{"checks":{"d":{"runs":[{"passed":true}]}}}"#,
        "{",
    ] {
        publish_flaky(&sandbox, invalid);
        let (history, err) = history_of(&sandbox);
        assert_eq!(history, Ok(FlakyHistory::default()), "{invalid}");
        assert_eq!(err, DEGRADED, "{invalid}");
    }
}

#[test]
fn a_flaky_history_that_cannot_be_fetched_is_an_error() {
    let sandbox = Sandbox::new();
    sandbox.repo();
    let unreachable = sandbox.path("unreachable.git");
    sandbox.git(
        "repo",
        &["remote", "set-url", "origin", unreachable.to_str().unwrap()],
    );

    let (history, err) = history_of(&sandbox);

    assert_eq!(
        history,
        Err("flaky-history: without the record, every VM check would be measured again; fix the fetch instead".to_owned())
    );
    assert!(
        err.starts_with("ci-badges: the badges branch of origin could not be fetched"),
        "{err}"
    );
}
