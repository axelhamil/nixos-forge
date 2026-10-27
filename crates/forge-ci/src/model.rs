use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Number;

pub const CACHED: &str = "cached";
pub const COMPILES_A_KERNEL: &str = "compiles a kernel";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedJob {
    pub check: String,
    pub system: String,
    #[serde(default)]
    pub runner: String,
    #[serde(default)]
    pub vm: bool,
    #[serde(default, rename = "baseInput")]
    pub base_input: Option<String>,
    #[serde(default)]
    pub timeout: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedJob {
    pub check: String,
    pub system: String,
    #[serde(default)]
    pub vm: bool,
    #[serde(default)]
    pub reason: String,
}

pub type HostChecks = BTreeMap<String, Vec<String>>;

pub type Descriptions = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BootTime {
    pub total: Option<f64>,
    pub userspace: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    #[serde(default)]
    pub check: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub excerpt: Vec<String>,
    #[serde(default, rename = "bootTime")]
    pub boot_time: Option<BootTime>,
    #[serde(default)]
    pub screenshots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Badge {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u8,
    pub label: String,
    pub message: String,
    pub color: String,
}

impl Badge {
    #[must_use]
    pub fn new(label: &str, message: &str, color: &str) -> Self {
        Self {
            schema_version: 1,
            label: label.to_owned(),
            message: message.to_owned(),
            color: color.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlakyRun {
    pub passed: bool,
    pub seconds: Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlakyRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub runs: Vec<FlakyRun>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlakyHistory {
    #[serde(default)]
    pub tree: Option<String>,
    pub checks: BTreeMap<String, FlakyRecord>,
}

#[must_use]
pub fn key(system: &str, check: &str) -> String {
    format!("{system}.{check}")
}

#[must_use]
pub fn json_or_none<T: DeserializeOwned>(text: Option<&str>) -> Option<T> {
    serde_json::from_str(text?).ok()
}

#[must_use]
pub fn env_json<T: DeserializeOwned>(name: &str) -> Option<T> {
    json_or_none(env::var(name).ok().as_deref())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Passed,
    Failed,
    DidNotFinish,
    Skipped,
    Other(String),
}

impl Status {
    #[must_use]
    pub fn from_outcome(outcome: &str) -> Self {
        match outcome {
            "passed" => Self::Passed,
            "failed" => Self::Failed,
            other => Self::Other(other.to_owned()),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::DidNotFinish => "did not finish",
            Self::Skipped => "skipped",
            Self::Other(outcome) => outcome,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStatus<'a> {
    pub check: &'a str,
    pub system: &'a str,
    pub vm: bool,
    pub status: Status,
}

#[must_use]
pub fn by_key(results: &[CheckResult]) -> HashMap<String, &CheckResult> {
    results
        .iter()
        .map(|result| (key(&result.system, &result.check), result))
        .collect()
}

#[must_use]
pub fn statuses<'a>(
    planned: &'a [PlannedJob],
    skipped: &'a [SkippedJob],
    results: &[CheckResult],
) -> Vec<JobStatus<'a>> {
    let outcomes = by_key(results);
    let planned = planned.iter().map(|job| {
        let outcome = outcomes
            .get(&key(&job.system, &job.check))
            .and_then(|result| result.outcome.as_deref());
        JobStatus {
            check: &job.check,
            system: &job.system,
            vm: job.vm,
            status: outcome.map_or(Status::DidNotFinish, Status::from_outcome),
        }
    });
    let skipped = skipped.iter().map(|job| JobStatus {
        check: &job.check,
        system: &job.system,
        vm: job.vm,
        status: if job.reason == CACHED {
            Status::Passed
        } else {
            Status::Skipped
        },
    });
    planned.chain(skipped).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    NotBuilt,
    LeftOver(String),
    Identical,
    NotFinished,
    Differs,
}

pub const REPRODUCIBILITY_HEADING: &str = "## Reproducibility";
pub const REPRODUCIBILITY_INTRO: &str = "Each system is built, then built again with `nix build --rebuild` and compared. A difference is expected now and then and fails nothing.";

const NOT_BUILT: &str = "the system did not build, see the job log.";
const LEFT_OVER: &str = ".check` is left from an earlier run and cannot be removed, so a difference could not be told apart; remove it as root and run again.";
const IDENTICAL: &str = "the rebuild is identical.";
const NOT_FINISHED: &str = "the rebuild did not finish, see the job log.";
const DIFFERS: &str = "the rebuild differs from the first build.";

impl Verdict {
    #[must_use]
    pub fn line(&self, host: &str) -> String {
        let verdict = match self {
            Self::NotBuilt => NOT_BUILT.to_owned(),
            Self::LeftOver(path) => format!("`{path}{LEFT_OVER}"),
            Self::Identical => IDENTICAL.to_owned(),
            Self::NotFinished => NOT_FINISHED.to_owned(),
            Self::Differs => DIFFERS.to_owned(),
        };
        format!("- `{host}`: {verdict}")
    }

    #[must_use]
    pub fn parse(line: &str) -> Option<(&str, Self)> {
        let (host, verdict) = line.strip_prefix("- `")?.split_once("`: ")?;
        if host.is_empty() || host.contains('`') {
            return None;
        }

        let verdict = match verdict {
            NOT_BUILT => Self::NotBuilt,
            IDENTICAL => Self::Identical,
            NOT_FINISHED => Self::NotFinished,
            DIFFERS => Self::Differs,
            other => {
                let path = other.strip_prefix('`')?;
                Self::LeftOver(path.strip_suffix(LEFT_OVER).unwrap_or(path).to_owned())
            }
        };
        Some((host, verdict))
    }

    #[must_use]
    pub fn finished(&self) -> bool {
        matches!(self, Self::Identical | Self::Differs)
    }

    #[must_use]
    pub fn rebuilt(&self) -> bool {
        matches!(self, Self::Identical | Self::NotFinished | Self::Differs)
    }
}

#[must_use]
pub fn jq_number(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_owned();
    }

    let scientific = format!("{:e}", value.abs());
    let sign = if value < 0.0 { "-" } else { "" };
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits = mantissa.replace('.', "");
    let exponent: i64 = exponent.parse().unwrap_or_default();
    let point = exponent + 1;
    let count = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    if point <= -4 || point > count + 15 {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let exponent_sign = if exponent < 0 { '-' } else { '+' };
        return format!(
            "{sign}{first}{fraction}e{exponent_sign}{:02}",
            exponent.unsigned_abs()
        );
    }

    if point <= 0 {
        let zeros = "0".repeat(usize::try_from(-point).unwrap_or_default());
        return format!("{sign}0.{zeros}{digits}");
    }

    let point = usize::try_from(point).unwrap_or_default();
    if point >= digits.len() {
        return format!("{sign}{digits}{}", "0".repeat(point - digits.len()));
    }

    format!("{sign}{}.{}", &digits[..point], &digits[point..])
}

#[must_use]
pub fn one_decimal(value: f64) -> String {
    let rounded = jq_number((value * 10.0).round() / 10.0);
    if rounded.contains('.') {
        return rounded;
    }

    format!("{rounded}.0")
}

#[must_use]
pub fn seconds(value: f64) -> String {
    format!("{} s", one_decimal(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_joins_the_system_and_the_check_with_a_dot() {
        assert_eq!(
            key("x86_64-linux", "example-boots"),
            "x86_64-linux.example-boots"
        );
    }

    #[test]
    fn json_of_the_expected_type_is_kept() {
        let planned: Option<Vec<PlannedJob>> =
            json_or_none(Some(r#"[{"check":"docs","system":"x86_64-linux"}]"#));
        assert_eq!(planned.unwrap()[0].check, "docs");
        let hosts: Option<HostChecks> = json_or_none(Some(r#"{"example":["example-boots"]}"#));
        assert_eq!(hosts.unwrap()["example"], ["example-boots"]);
    }

    #[test]
    fn absent_empty_malformed_or_mistyped_json_is_none() {
        for text in [
            None,
            Some(""),
            Some("  "),
            Some("null"),
            Some("[1,"),
            Some(r#"{"a":1}"#),
        ] {
            assert_eq!(json_or_none::<Vec<SkippedJob>>(text), None, "{text:?}");
        }
        assert_eq!(json_or_none::<HostChecks>(Some("[]")), None);
    }

    #[test]
    fn an_unset_variable_is_none() {
        assert_eq!(env_json::<HostChecks>("FORGE_CI_NEVER_SET_IN_A_TEST"), None);
    }

    fn planned(check: &str, system: &str, vm: bool) -> PlannedJob {
        PlannedJob {
            check: check.to_owned(),
            system: system.to_owned(),
            vm,
            ..PlannedJob::default()
        }
    }

    fn skipped(check: &str, reason: &str) -> SkippedJob {
        SkippedJob {
            check: check.to_owned(),
            system: "x86_64-linux".to_owned(),
            vm: true,
            reason: reason.to_owned(),
        }
    }

    fn result(check: &str, system: &str, outcome: Option<&str>) -> CheckResult {
        CheckResult {
            check: check.to_owned(),
            system: system.to_owned(),
            outcome: outcome.map(str::to_owned),
            ..CheckResult::default()
        }
    }

    #[test]
    fn a_planned_check_takes_its_outcome_or_did_not_finish() {
        let planned = [
            planned("example-toplevel", "x86_64-linux", false),
            planned("example-boots", "x86_64-linux", true),
            planned("example-boots", "aarch64-linux", true),
            planned("docs", "x86_64-linux", false),
            planned("feature-ssh", "x86_64-linux", true),
        ];
        let results = [
            result("example-toplevel", "x86_64-linux", Some("passed")),
            result("example-boots", "x86_64-linux", Some("failed")),
            result("docs", "x86_64-linux", Some("timed out")),
            result("feature-ssh", "x86_64-linux", None),
        ];
        let statuses: Vec<Status> = statuses(&planned, &[], &results)
            .into_iter()
            .map(|job| job.status)
            .collect();
        assert_eq!(
            statuses,
            [
                Status::Passed,
                Status::Failed,
                Status::DidNotFinish,
                Status::Other("timed out".to_owned()),
                Status::DidNotFinish,
            ]
        );
    }

    #[test]
    fn a_skipped_check_passed_when_cached_and_is_skipped_otherwise() {
        let skipped = [
            skipped("feature-ssh", CACHED),
            skipped("example-pi-sd-image", COMPILES_A_KERNEL),
        ];
        let results = [result("feature-ssh", "x86_64-linux", Some("failed"))];
        let statuses = statuses(&[], &skipped, &results);
        assert_eq!(statuses[0].status, Status::Passed);
        assert_eq!(statuses[1].status, Status::Skipped);
        assert!(statuses[0].vm);
        assert_eq!(statuses[1].check, "example-pi-sd-image");
    }

    #[test]
    fn the_last_result_of_a_check_wins() {
        let results = [
            result("docs", "x86_64-linux", Some("failed")),
            result("docs", "x86_64-linux", Some("passed")),
        ];
        let planned = [planned("docs", "x86_64-linux", false)];
        let statuses = statuses(&planned, &[], &results);
        assert_eq!(statuses[0].status, Status::Passed);
    }

    #[test]
    fn a_status_reads_as_the_shell_wrote_it() {
        let texts: Vec<String> = [
            Status::Passed,
            Status::Failed,
            Status::DidNotFinish,
            Status::Skipped,
            Status::Other("timed out".to_owned()),
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            texts,
            ["passed", "failed", "did not finish", "skipped", "timed out"]
        );
        assert_eq!(Status::from_outcome("failed"), Status::Failed);
    }

    #[test]
    fn a_badge_is_a_shields_endpoint() {
        let badge =
            serde_json::to_string(&Badge::new("checks", "3/3 passing", "brightgreen")).unwrap();
        assert_eq!(
            badge,
            r#"{"schemaVersion":1,"label":"checks","message":"3/3 passing","color":"brightgreen"}"#
        );
    }

    #[test]
    fn a_planned_job_keeps_the_field_order_and_null_base_input_of_the_plan() {
        let job = PlannedJob {
            runner: "ubuntu-24.04".to_owned(),
            timeout: 60,
            ..planned("docs", "x86_64-linux", false)
        };
        assert_eq!(
            serde_json::to_string(&job).unwrap(),
            r#"{"check":"docs","system":"x86_64-linux","runner":"ubuntu-24.04","vm":false,"baseInput":null,"timeout":60}"#
        );
    }

    #[test]
    fn a_flaky_history_needs_its_checks_and_typed_runs() {
        let valid: Option<FlakyHistory> = json_or_none(Some(
            r#"{"checks":{"/nix/store/a.drv":{"check":"a","system":"x","runs":[{"passed":true,"seconds":12}]}}}"#,
        ));
        let valid = valid.unwrap();
        assert_eq!(valid.tree, None);
        assert_eq!(
            serde_json::to_string(&valid.checks["/nix/store/a.drv"]).unwrap(),
            r#"{"check":"a","system":"x","runs":[{"passed":true,"seconds":12}]}"#
        );
        for text in [
            r#"{"tree":null}"#,
            r#"{"tree":1,"checks":{}}"#,
            r#"{"checks":{"d":{"runs":[{"passed":"yes","seconds":1}]}}}"#,
            r#"{"checks":{"d":{"runs":[{"passed":true}]}}}"#,
        ] {
            assert_eq!(json_or_none::<FlakyHistory>(Some(text)), None, "{text}");
        }
    }

    #[test]
    fn every_reproducibility_line_parses_back_to_its_verdict() {
        let verdicts = [
            Verdict::NotBuilt,
            Verdict::LeftOver("/nix/store/abc-nixos-system".to_owned()),
            Verdict::Identical,
            Verdict::NotFinished,
            Verdict::Differs,
        ];
        for verdict in verdicts {
            let line = verdict.line("example");
            assert_eq!(Verdict::parse(&line), Some(("example", verdict)), "{line}");
        }
    }

    #[test]
    fn the_reproducibility_lines_keep_the_shell_text() {
        assert_eq!(
            Verdict::Differs.line("example"),
            "- `example`: the rebuild differs from the first build."
        );
        assert_eq!(
            Verdict::LeftOver("/nix/store/p".to_owned()).line("example"),
            "- `example`: `/nix/store/p.check` is left from an earlier run and cannot be removed, so a difference could not be told apart; remove it as root and run again."
        );
    }

    #[test]
    fn other_lines_are_no_verdict() {
        for line in [
            REPRODUCIBILITY_HEADING,
            "- ``: the rebuild is identical.",
            "- `example`: something else.",
            "<summary>diffoscope of example, first 200 lines</summary>",
        ] {
            assert_eq!(Verdict::parse(line), None, "{line}");
        }
    }

    #[test]
    fn a_verdict_tells_whether_the_rebuild_finished_and_ran() {
        assert!(Verdict::Identical.finished() && Verdict::Identical.rebuilt());
        assert!(Verdict::Differs.finished() && Verdict::Differs.rebuilt());
        assert!(!Verdict::NotFinished.finished() && Verdict::NotFinished.rebuilt());
        assert!(!Verdict::NotBuilt.finished() && !Verdict::NotBuilt.rebuilt());
        assert!(!Verdict::LeftOver(String::new()).finished());
        assert!(!Verdict::LeftOver(String::new()).rebuilt());
    }
}
