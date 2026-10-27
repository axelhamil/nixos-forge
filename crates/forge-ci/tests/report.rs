#[path = "../../forge/tests/common/mod.rs"]
mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::{Value, json};
use tempfile::TempDir;

const RUN_URL: &str = "https://github.com/owner/repo/actions/runs/42";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/runs")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture(name)).unwrap()
}

fn planned() -> Vec<Value> {
    serde_json::from_str(&read_fixture("planned.json")).unwrap()
}

fn excerpt(name: &str) -> Value {
    serde_json::from_str(&read_fixture(&format!("excerpts/{name}.json"))).unwrap()
}

fn write_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

struct Scenario {
    dir: TempDir,
    planned: Vec<Value>,
    env: BTreeMap<&'static str, String>,
}

impl Scenario {
    fn empty() -> Self {
        let env = BTreeMap::from([
            ("PLAN_RESULT", "success".to_owned()),
            ("HOST_CHECKS", read_fixture("host-checks.json")),
            ("DESCRIPTIONS", read_fixture("descriptions.json")),
            ("PLANNED", read_fixture("planned.json")),
            ("SKIPPED", "[]".to_owned()),
            ("CACHE_OUTCOME", "success".to_owned()),
            ("LINT_RESULT", "success".to_owned()),
            ("RUN_URL", RUN_URL.to_owned()),
        ]);
        Self {
            dir: tempfile::tempdir().unwrap(),
            planned: planned(),
            env,
        }
    }

    fn passing() -> Self {
        Self::passing_with(planned())
    }

    fn passing_with(planned: Vec<Value>) -> Self {
        let mut scenario = Self::empty();
        scenario.planned = planned;
        scenario
            .env
            .insert("PLANNED", serde_json::to_string(&scenario.planned).unwrap());
        for job in &scenario.planned {
            let result = json!({
                "check": job["check"],
                "system": job["system"],
                "outcome": "passed",
                "excerpt": [],
            });
            scenario.write_result(&job["system"], &job["check"], &result);
        }
        scenario
    }

    fn results(&self) -> PathBuf {
        self.dir.path().join("results")
    }

    fn result_path(&self, system: &str, check: &str) -> PathBuf {
        self.results().join(format!("{system}-{check}.json"))
    }

    fn write_result(&self, system: &Value, check: &Value, result: &Value) {
        let path = self.result_path(system.as_str().unwrap(), check.as_str().unwrap());
        write_file(&path, &serde_json::to_string_pretty(result).unwrap());
    }

    fn write(&self, relative: &str, text: &str) {
        write_file(&self.results().join(relative), text);
    }

    fn remove(&self, system: &str, check: &str) {
        fs::remove_file(self.result_path(system, check)).unwrap();
    }

    fn update(&self, system: &str, check: &str, change: impl FnOnce(&mut Value)) {
        let path = self.result_path(system, check);
        let mut result: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        change(&mut result);
        write_file(&path, &serde_json::to_string_pretty(&result).unwrap());
    }

    fn fail(&self, check: &str, log: &str) -> &Self {
        self.fail_on("x86_64-linux", check, log)
    }

    fn fail_on(&self, system: &str, check: &str, log: &str) -> &Self {
        self.update(system, check, |result| {
            result["outcome"] = json!("failed");
            result["excerpt"] = excerpt(log);
        });
        self
    }

    fn set(mut self, name: &'static str, value: &str) -> Self {
        self.env.insert(name, value.to_owned());
        self
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = common::sandboxed(
            Path::new(env!("CARGO_BIN_EXE_forge-ci")),
            self.dir.path(),
            self.dir.path(),
            self.dir.path(),
        );
        command
            .arg("report")
            .args(args)
            .env("RESULTS_DIR", self.results())
            .envs(&self.env);
        command.output().unwrap()
    }

    fn report(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "ci-report exits {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(
            report
                .bytes()
                .all(|byte| byte == b'\n' || (b' '..=b'~').contains(&byte)),
            "the report has a character outside plain ASCII (dash, emoji): {report}"
        );
        report
    }
}

fn lines(report: &str) -> Vec<&str> {
    report.lines().collect()
}

type Side = (f64, f64, u64, u64, u64);

fn perf_side((total, userspace, memory, closure, services): Side) -> Value {
    let boot = json!({
        "total": total,
        "userspace": userspace,
        "memory": memory,
        "services": services,
        "cpu": 7500,
        "memoryBandwidth": 50000,
        "randomRead": 50000,
    });
    json!({ "min": boot, "median": boot, "closure": closure, "samples": [] })
}

fn perf_pair(base: Side, current: Side) -> Value {
    json!({ "base": perf_side(base), "current": perf_side(current) })
}

fn steady() -> Value {
    let side = (7.0, 3.0, 400, 2_147_483_648, 20);
    perf_pair(side, side)
}

fn perf_run(example: &Value, server: &Value) -> Scenario {
    let scenario = Scenario::passing();
    let measures =
        json!({ "rounds": 5, "hosts": { "example": example, "example-server": server } });
    scenario.write("perf/x86_64-linux.json", &measures.to_string());
    scenario
}

fn with_median(mut measures: Value, side: &str, values: &Value) -> Value {
    for (name, value) in values.as_object().unwrap() {
        measures[side]["median"][name] = value.clone();
    }
    measures
}

#[test]
fn a_green_run_lists_every_check_and_the_package_changes() {
    let scenario = Scenario::passing();
    scenario.write(
        "closure-x86_64-linux.md",
        "### Closure diff (x86_64-linux)\n\n#### example\n```\nnew host\n```\n",
    );
    insta::assert_snapshot!("all-green", scenario.report(&[]));
}

#[test]
fn a_failed_boot_test_names_its_machine_cause_and_log() {
    let scenario = Scenario::passing();
    scenario.fail("example-server-boots", "assertion");
    insta::assert_snapshot!("one-failed", scenario.report(&[]));
}

#[test]
fn a_missing_result_did_not_finish() {
    let scenario = Scenario::passing();
    scenario.remove("x86_64-linux", "example-disk");
    insta::assert_snapshot!("missing", scenario.report(&[]));
}

#[test]
fn an_unreachable_cache_is_reported() {
    let scenario = Scenario::passing().set("CACHE_OUTCOME", "failure");
    insta::assert_snapshot!("cache-down", scenario.report(&[]));
}

#[test]
fn skipped_checks_say_why() {
    let kept: Vec<Value> = planned()
        .into_iter()
        .filter(|job| job["check"] != "example-pi-sd-image" && job["check"] != "feature-ssh")
        .collect();
    let scenario = Scenario::passing()
        .set("SKIPPED", r#"[{"check":"example-pi-sd-image","system":"aarch64-linux","reason":"compiles a kernel"},{"check":"feature-ssh","system":"x86_64-linux","reason":"cached"}]"#)
        .set("PLANNED", &serde_json::to_string(&kept).unwrap());
    scenario.remove("aarch64-linux", "example-pi-sd-image");
    scenario.remove("x86_64-linux", "feature-ssh");
    insta::assert_snapshot!("kernel", scenario.report(&[]));
}

#[test]
fn a_failed_module_and_lint_affect_no_machine() {
    let scenario = Scenario::passing().set("LINT_RESULT", "failure");
    scenario.fail("feature-ssh", "eval-error");
    insta::assert_snapshot!("module-failed", scenario.report(&[]));
}

#[test]
fn each_known_cause_is_diagnosed() {
    for (name, check, log) in [
        ("timeout", "example-boots", "timeout"),
        ("assertion-only", "example-boots", "assertion-only"),
        ("guest-noise", "example-boots", "guest-noise"),
        ("compile", "example-toplevel", "compile-error"),
        ("hash", "example-toplevel", "hash-mismatch"),
        ("substitute", "example-toplevel", "substitute"),
        ("download-warnings", "example-boots", "download-warnings"),
        (
            "substitute-and-assertion",
            "example-boots",
            "substitute-and-assertion",
        ),
        (
            "substitute-and-compile",
            "example-toplevel",
            "substitute-and-compile",
        ),
        ("no-space", "example-toplevel", "no-space"),
    ] {
        let scenario = Scenario::passing();
        scenario.fail(check, log);
        insta::assert_snapshot!(name, scenario.report(&[]));
    }
}

#[test]
fn no_plan_builds_nothing() {
    let scenario = Scenario::empty().set("PLANNED", "");
    insta::assert_snapshot!("no-plan", scenario.report(&[]));
}

#[test]
fn a_flake_that_does_not_evaluate_points_to_the_plan_job() {
    let mut scenario = Scenario::empty();
    for name in [
        "PLANNED",
        "SKIPPED",
        "CACHE_OUTCOME",
        "HOST_CHECKS",
        "DESCRIPTIONS",
    ] {
        scenario = scenario.set(name, "");
    }
    let scenario = scenario
        .set("PLAN_RESULT", "failure")
        .set("LINT_RESULT", "failure");
    insta::assert_snapshot!("plan-failed", scenario.report(&[]));
}

#[test]
fn an_excerpt_fence_is_longer_than_the_backticks_it_holds() {
    let scenario = Scenario::passing();
    scenario.fail("feature-ssh", "fence");
    let report = scenario.report(&[]);
    assert_eq!(
        lines(&report)
            .iter()
            .filter(|line| **line == "`````")
            .count(),
        2
    );
    insta::assert_snapshot!("fence", report);
}

#[test]
fn check_names_are_escaped_in_the_tables() {
    let mut planned = planned();
    for check in ["system-a|b", "app-x`y"] {
        planned.push(json!({ "check": check, "system": "x86_64-linux" }));
    }
    let scenario = Scenario::passing_with(planned);
    insta::assert_snapshot!("escape", scenario.report(&[]));
}

#[test]
fn a_check_on_two_systems_names_each_system() {
    let mut planned = planned();
    planned.push(json!({ "check": "feature-ssh", "system": "aarch64-linux" }));
    let scenario = Scenario::passing_with(planned);
    scenario.fail_on("aarch64-linux", "feature-ssh", "eval-error");
    insta::assert_snapshot!("two-systems", scenario.report(&[]));
}

#[test]
fn boot_times_and_screenshots_get_their_sections() {
    let scenario = Scenario::passing();
    scenario.update("x86_64-linux", "example-boots", |result| {
        result["bootTime"] = json!({ "total": 7.04, "userspace": 3.0 });
    });
    scenario.update("x86_64-linux", "feature-ssh", |result| {
        result["screenshots"] = json!(["desktop.png"]);
    });
    scenario.write(
        "base/boot-time.json",
        r#"{ "example": { "total": 6.8, "userspace": null }, "example-server": { "total": 9, "userspace": 4.25 } }"#,
    );
    insta::assert_snapshot!("boot-time", scenario.report(&[]));
}

#[test]
fn an_unreadable_base_boot_time_is_not_available() {
    let scenario = Scenario::passing();
    scenario.update("x86_64-linux", "example-boots", |result| {
        result["bootTime"] = json!({ "total": 7.04, "userspace": 3.0 });
    });
    scenario.write("base/boot-time.json", "not json\n");
    insta::assert_snapshot!("no-base", scenario.report(&[]));
}

#[test]
fn steady_measures_cross_no_threshold() {
    let mut noisy_median = steady();
    noisy_median["current"]["median"]["total"] = json!(9.9);
    let scenario = perf_run(&steady(), &noisy_median);
    insta::assert_snapshot!("perf-steady", scenario.report(&[]));
}

#[test]
fn regressions_past_both_thresholds_are_flagged_without_turning_red() {
    let scenario = perf_run(
        &perf_pair(
            (7.0, 3.0, 400, 2_147_483_648, 20),
            (7.8, 3.2, 448, 2_147_483_648, 21),
        ),
        &perf_pair(
            (7.0, 3.0, 400, 2_147_483_648, 20),
            (6.0, 3.0, 400, 2_361_393_152, 19),
        ),
    );
    insta::assert_snapshot!("perf-regressed", scenario.report(&[]));
}

#[test]
fn benchmark_drops_are_flagged() {
    let scenario = perf_run(
        &with_median(
            steady(),
            "current",
            &json!({ "cpu": 7000, "memoryBandwidth": 60000, "randomRead": 46000 }),
        ),
        &with_median(
            steady(),
            "current",
            &json!({ "cpu": 7600, "memoryBandwidth": 45000, "randomRead": 42000 }),
        ),
    );
    insta::assert_snapshot!("perf-benchmarks", scenario.report(&[]));
}

#[test]
fn a_benchmark_drop_under_its_absolute_threshold_is_not_flagged() {
    let measures = with_median(
        with_median(steady(), "base", &json!({ "cpu": 1000 })),
        "current",
        &json!({ "cpu": 940 }),
    );
    let scenario = perf_run(&measures, &steady());
    insta::assert_snapshot!("perf-benchmarks-small", scenario.report(&[]));
}

#[test]
fn small_absolute_changes_are_not_flagged() {
    let scenario = perf_run(
        &perf_pair(
            (2.0, 1.0, 200, 209_715_200, 20),
            (2.3, 1.0, 225, 241_172_480, 20),
        ),
        &perf_pair(
            (7.0, 3.0, 400, 3_155_385_672, 20),
            (7.0, 3.0, 400, 3_154_346_392, 20),
        ),
    );
    insta::assert_snapshot!("perf-small", scenario.report(&[]));
}

#[test]
fn partial_measures_are_not_available() {
    let example = json!({ "current": perf_side((7.0, 3.0, 400, 2_147_483_648, 20)) });
    let scenario = perf_run(&example, &json!("not an object"));
    insta::assert_snapshot!("perf-partial", scenario.report(&[]));
}

#[test]
fn machines_the_base_cannot_measure_are_listed() {
    let scenario = perf_run(&steady(), &steady());
    let measures = json!({
        "rounds": 5,
        "hosts": { "example": steady(), "example-server": steady() },
        "skipped": { "laptop": "absent", "pi": "no-vm" },
    });
    scenario.write("perf/x86_64-linux.json", &measures.to_string());
    insta::assert_snapshot!("perf-fork-host", scenario.report(&[]));
}

#[test]
fn a_run_measuring_no_machine_lists_the_unmeasured_ones() {
    let scenario = Scenario::passing();
    scenario.write(
        "perf/x86_64-linux.json",
        r#"{ "rounds": 5, "hosts": {}, "skipped": { "laptop": "absent" } }"#,
    );
    insta::assert_snapshot!("perf-none-on-base", scenario.report(&[]));
}

#[test]
fn unreadable_measures_show_no_performance_section() {
    let scenario = Scenario::passing();
    scenario.write("perf/x86_64-linux.json", "not json\n");
    let report = scenario.report(&[]);
    assert!(!report.contains("### Performance"));
    insta::assert_snapshot!("perf-unreadable", report);
}

fn long_closure() -> Scenario {
    let scenario = Scenario::passing();
    let mut closure = String::from("### Closure diff (x86_64-linux)\n\n#### example\n```\n");
    for package in 1..=20000 {
        writeln!(closure, "[U.] package-{package}: 1.0 -> 2.0").unwrap();
    }
    closure.push_str("```\n");
    scenario.write("closure-x86_64-linux.md", &closure);
    scenario
}

#[test]
fn a_long_comment_is_cut_with_its_blocks_closed() {
    let report = long_closure().report(&["--comment"]);
    let lines = lines(&report);

    assert!(report.len() <= 65536, "{} bytes", report.len());
    assert_eq!(
        lines.last(),
        Some(
            &"The report is too long for a comment: read it in full in the [run summary](https://github.com/owner/repo/actions/runs/42)."
        )
    );
    assert_eq!(
        lines.iter().filter(|line| line.starts_with("```")).count() % 2,
        0
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("<details"))
            .count(),
        lines
            .iter()
            .filter(|line| line.starts_with("</details>"))
            .count()
    );
    insta::assert_snapshot!("long-comment-end", lines[lines.len() - 8..].join("\n"));
}

#[test]
fn a_long_summary_keeps_what_fits_its_limit() {
    let report = long_closure().report(&[]);
    assert!(
        report
            .lines()
            .any(|line| line == "[U.] package-20000: 1.0 -> 2.0")
    );
    assert!(!report.contains("The report is too long"));
}

#[test]
fn the_note_sits_under_the_verdict_once() {
    let green = Scenario::passing().report(&[]);
    assert_eq!(
        lines(&green)[2],
        "Nothing to fix: this commit is ready for the machines."
    );

    let scenario = Scenario::passing();
    scenario.fail("example-server-boots", "assertion");
    let red = scenario.report(&["--comment"]);
    assert_eq!(
        lines(&red)[2],
        "Fix what is listed below and push again: CI runs on every push."
    );
    assert_eq!(red.matches("Fix what is listed below").count(), 1);
    insta::assert_snapshot!("red-note", red);

    let no_plan = Scenario::empty()
        .set("PLANNED", "")
        .set("PLAN_RESULT", "failure");
    insta::assert_snapshot!("red-note-no-plan", no_plan.report(&[]));
}

#[test]
fn an_unknown_argument_is_a_usage_error() {
    let output = Scenario::passing().run(&["--summary"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"usage: ci-report [--comment]\n");
}
