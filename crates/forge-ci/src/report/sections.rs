use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Number, Value};

use super::Inputs;
use super::causes::{Causes, Diagnosis};
use crate::model::{CACHED, COMPILES_A_KERNEL, key};

const READY: &str = "Nothing to fix: this commit is ready for the machines.";
const TO_FIX: &str = "Fix what is listed below and push again: CI runs on every push.";
const OWN_TEST: &str = "its own test passes";
const PASSED: &str = "passed";
const FAILED: &str = "failed";
const DID_NOT_FINISH: &str = "did not finish";
const SKIPPED: &str = "skipped";
const TREEFMT: &str = "treefmt";
const MEBIBYTE: f64 = 1_048_576.0;

const NOT_FINISHED: Diagnosis<'static> = Diagnosis {
    cause: "the job stopped before it wrote a result: it was cancelled or hit its time limit",
    todo: "rerun the job; if it stops again, a test hangs",
};
const NOT_FORMATTED: Diagnosis<'static> = Diagnosis {
    cause: "the code is not formatted or a linter found a problem",
    todo: "run `nix fmt`, then commit the result",
};
const UNKNOWN: Diagnosis<'static> = Diagnosis {
    cause: "no known cause",
    todo: "open the full log",
};

const PERF_INTRO_START: &str = "The base revision and this change, each booted ";
const PERF_INTRO_MIDDLE: &str =
    " times in turn in one VM job. Boot times and idle RAM compare the best of the ";
const PERF_INTRO_END: &str = " boots of each side; running services and the benchmarks run after each boot (CPU, memory bandwidth, disk) their median; the closure that of the real system. A metric is flagged only past both its relative and its absolute threshold, a benchmark when it drops. The benchmark thresholds come from the noise of local runs and a shared CI runner is noisier: read their trend over several runs, not one value. Informational: it never blocks the merge.";

fn text<'a>(object: &'a Map<String, Value>, field: &str) -> &'a str {
    object
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn object(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn outcome(result: &Map<String, Value>) -> Option<String> {
    match result.get("outcome")? {
        Value::Null | Value::Bool(false) => None,
        Value::String(outcome) => Some(outcome.clone()),
        other => Some(other.to_string()),
    }
}

fn jq_number(value: f64) -> String {
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

fn one_decimal(value: f64) -> String {
    let rounded = jq_number((value * 10.0).round() / 10.0);
    if rounded.contains('.') {
        return rounded;
    }

    format!("{rounded}.0")
}

fn seconds(value: Option<f64>) -> String {
    value.map_or_else(
        || "n/a".to_owned(),
        |value| format!("{} s", one_decimal(value)),
    )
}

fn plural<'a>(count: usize, one: &'a str, many: &'a str) -> &'a str {
    if count == 1 { one } else { many }
}

fn names(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

fn code(text: &str) -> String {
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or_default();
    let delimiter = "`".repeat(longest + 1);
    if text.starts_with('`') || text.ends_with('`') {
        return format!("{delimiter} {text} {delimiter}");
    }

    format!("{delimiter}{text}{delimiter}")
}

fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|")
}

#[derive(Debug, Clone)]
struct Row {
    check: String,
    system: String,
    status: String,
    reason: Option<String>,
    excerpt: Vec<String>,
    place: String,
}

impl Row {
    fn bad(&self) -> bool {
        self.status == FAILED || self.status == DID_NOT_FINISH
    }

    fn cached(&self) -> bool {
        self.status == SKIPPED && self.reason.as_deref() == Some(CACHED)
    }

    fn cell(&self) -> String {
        if self.status != SKIPPED {
            return self.status.clone();
        }

        let why = if self.reason.as_deref() == Some(CACHED) {
            "already passed"
        } else {
            "runs after the merge"
        };
        format!("skipped: {why}")
    }
}

struct BootRow<'a> {
    host: &'a str,
    current: &'a Map<String, Value>,
    base: &'a Map<String, Value>,
}

struct Report<'a> {
    inputs: &'a Inputs,
    causes: &'a Causes,
}

impl Report<'_> {
    fn description(&self, check: &str) -> &str {
        self.inputs
            .descriptions
            .get(check)
            .map_or(OWN_TEST, String::as_str)
    }

    fn table(&self, rows: &[&Row]) -> Vec<String> {
        let mut lines = vec![
            "| Check | What it proves | Result |".to_owned(),
            "|---|---|---|".to_owned(),
        ];
        lines.extend(rows.iter().map(|row| {
            format!(
                "| {}{} | {} | {} |",
                escape_cell(&code(&row.check)),
                row.place,
                escape_cell(self.description(&row.check)),
                row.cell()
            )
        }));
        lines
    }

    fn diagnosis(&self, row: &Row) -> Diagnosis<'static> {
        if row.status == DID_NOT_FINISH {
            return NOT_FINISHED;
        }

        if row.check == TREEFMT {
            return NOT_FORMATTED;
        }

        self.causes.diagnose(&row.excerpt).unwrap_or(UNKNOWN)
    }

    fn failure(&self, row: &Row) -> Vec<String> {
        let diagnosis = self.diagnosis(row);
        let todo = diagnosis
            .todo
            .replace("<system>", &row.system)
            .replace("<check>", &row.check);
        let mut lines = vec![
            format!("#### {} ({})", code(&row.check), row.system),
            String::new(),
            format!("This check proves that {}.", self.description(&row.check)),
            String::new(),
            format!("- Cause: {}.", diagnosis.cause),
            format!("- What to do: {todo}."),
        ];
        if !row.excerpt.is_empty() {
            let longest = row
                .excerpt
                .iter()
                .flat_map(|line| line.split(|character| character != '`'))
                .map(|run| run.len() + 1)
                .fold(3, usize::max);
            let fence = "`".repeat(longest);
            lines.push(String::new());
            lines.push(fence.clone());
            lines.extend(row.excerpt.iter().cloned());
            lines.push(fence);
        }
        if !self.inputs.run.is_empty() {
            lines.push(String::new());
            lines.push(format!("[Open the full log]({})", self.inputs.run));
        }
        lines.push(String::new());
        lines
    }

    fn lint_status(&self) -> &'static str {
        match self.inputs.lint.as_str() {
            "success" => PASSED,
            "failure" => FAILED,
            "skipped" => SKIPPED,
            _ => DID_NOT_FINISH,
        }
    }

    fn listed(&self, planned: &[Value]) -> Vec<Row> {
        let mut by_check = HashMap::new();
        for result in &self.inputs.results {
            by_check.insert(key(text(result, "system"), text(result, "check")), result);
        }

        let planned = planned.iter().filter_map(Value::as_object).map(|job| {
            let check = text(job, "check").to_owned();
            let system = text(job, "system").to_owned();
            let result = by_check.get(&key(&system, &check));
            Row {
                status: result
                    .and_then(|result| outcome(result))
                    .unwrap_or_else(|| DID_NOT_FINISH.to_owned()),
                excerpt: strings(result.and_then(|result| result.get("excerpt"))),
                check,
                system,
                reason: None,
                place: String::new(),
            }
        });
        let skipped = self
            .inputs
            .skipped
            .iter()
            .filter_map(Value::as_object)
            .map(|job| Row {
                check: text(job, "check").to_owned(),
                system: text(job, "system").to_owned(),
                status: SKIPPED.to_owned(),
                reason: job.get("reason").and_then(Value::as_str).map(str::to_owned),
                excerpt: Vec::new(),
                place: String::new(),
            });
        let lint = Row {
            check: TREEFMT.to_owned(),
            system: "x86_64-linux".to_owned(),
            status: self.lint_status().to_owned(),
            reason: None,
            excerpt: Vec::new(),
            place: String::new(),
        };
        let mut rows: Vec<Row> = planned.chain(skipped).chain([lint]).collect();

        let mut counts: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            *counts.entry(row.check.clone()).or_default() += 1;
        }
        for row in &mut rows {
            if counts.get(&row.check).is_some_and(|count| *count > 1) {
                row.place = format!(" ({})", row.system);
            }
        }
        rows
    }

    fn planned(&self, planned: &[Value]) -> Vec<String> {
        let all = self.listed(planned);
        let hosts = &self.inputs.hosts;
        let host_check_names: BTreeSet<&str> =
            hosts.values().flatten().map(String::as_str).collect();
        let bad: Vec<&Row> = all.iter().filter(|row| row.bad()).collect();
        let bad_names: BTreeSet<&str> = bad.iter().map(|row| row.check.as_str()).collect();
        let bad_hosts: Vec<&str> = hosts
            .iter()
            .filter(|(_, checks)| {
                checks
                    .iter()
                    .any(|check| bad_names.contains(check.as_str()))
            })
            .map(|(host, _)| host.as_str())
            .collect();
        let cached = all.iter().filter(|row| row.cached()).count();
        let modules: Vec<&Row> = all
            .iter()
            .filter(|row| !host_check_names.contains(row.check.as_str()))
            .collect();

        let verdict = if bad.is_empty() {
            "**Everything works: every machine builds, boots and passed its tests.**".to_owned()
        } else {
            let what = if bad.iter().all(|row| row.status == FAILED) {
                FAILED
            } else if bad.iter().all(|row| row.status == DID_NOT_FINISH) {
                DID_NOT_FINISH
            } else {
                "failed or did not finish"
            };
            let affected = if bad_hosts.is_empty() {
                "no machine is affected, but CI stays red until it is fixed.".to_owned()
            } else {
                format!("{} would not get this change.", names(&bad_hosts))
            };
            format!(
                "**{} {} {what}: {affected}**",
                bad.len(),
                plural(bad.len(), "check", "checks")
            )
        };

        let mut lines = vec![
            verdict,
            String::new(),
            if bad.is_empty() { READY } else { TO_FIX }.to_owned(),
            String::new(),
        ];
        if self.inputs.cache == "failure" {
            lines.push(
                "The binary cache was unreachable: everything was built from scratch.".to_owned(),
            );
            lines.push(String::new());
        }
        if self
            .inputs
            .skipped
            .iter()
            .any(|job| job.get("reason").and_then(Value::as_str) == Some(COMPILES_A_KERNEL))
        {
            lines.push("Checks that compile a kernel run after the merge.".to_owned());
            lines.push(String::new());
        }
        if cached > 0 {
            lines.push(format!(
                "{cached} {}.",
                plural(
                    cached,
                    "check was skipped: it already passed with the same code",
                    "checks were skipped: they already passed with the same code"
                )
            ));
            lines.push(String::new());
        }
        for (host, checks) in hosts {
            let rows: Vec<&Row> = checks
                .iter()
                .flat_map(|name| all.iter().filter(move |row| &row.check == name))
                .collect();
            if rows.is_empty() {
                continue;
            }

            lines.push(format!("### {host}"));
            lines.push(String::new());
            lines.extend(self.table(&rows));
            lines.push(String::new());
        }
        lines.extend(self.modules(&modules));
        lines.extend(self.boot_times());
        lines.extend(self.performance());
        lines.extend(self.screenshots());
        if !bad.is_empty() {
            lines.push("### What went wrong".to_owned());
            lines.push(String::new());
            for row in &bad {
                lines.extend(self.failure(row));
            }
        }
        lines
    }

    fn modules(&self, modules: &[&Row]) -> Vec<String> {
        if modules.is_empty() {
            return Vec::new();
        }

        let counts: Vec<String> = [PASSED, FAILED, DID_NOT_FINISH, SKIPPED]
            .into_iter()
            .filter_map(|status| {
                let count = modules.iter().filter(|row| row.status == status).count();
                (count > 0).then(|| format!("{count} {status}"))
            })
            .collect();
        let open = if modules.iter().any(|row| row.bad()) {
            "<details open>"
        } else {
            "<details>"
        };
        let mut sorted = modules.to_vec();
        sorted.sort_by(|left, right| left.check.cmp(&right.check));
        let mut lines = vec![
            open.to_owned(),
            format!("<summary>Module checks: {}</summary>", counts.join(", ")),
            String::new(),
        ];
        lines.extend(self.table(&sorted));
        lines.extend([String::new(), "</details>".to_owned(), String::new()]);
        lines
    }

    fn boot_times(&self) -> Vec<String> {
        let empty = Map::new();
        let machines: Vec<BootRow> = self
            .inputs
            .hosts
            .iter()
            .filter(|(host, checks)| checks.contains(&format!("{host}-boots")))
            .map(|(host, _)| {
                let boots = format!("{host}-boots");
                let current = self
                    .inputs
                    .results
                    .iter()
                    .filter(|result| text(result, "check") == boots)
                    .find_map(|result| object(result.get("bootTime")))
                    .unwrap_or(&empty);
                let base = object(self.inputs.base_boot.get(host)).unwrap_or(&empty);
                BootRow {
                    host,
                    current,
                    base,
                }
            })
            .collect();
        if machines
            .iter()
            .all(|row| row.current.is_empty() && row.base.is_empty())
        {
            return Vec::new();
        }

        let measure = |side: &Map<String, Value>, field: &str| {
            seconds(side.get(field).and_then(Value::as_f64))
        };
        let mut lines = vec![
            "### Boot time".to_owned(),
            String::new(),
            "`systemd-analyze` in each boot test, on a shared CI VM: read the trend over several runs, not one value.".to_owned(),
            String::new(),
            "| Machine | Total | Total on the base | Userspace | Userspace on the base |".to_owned(),
            "|---|---|---|---|---|".to_owned(),
        ];
        lines.extend(machines.iter().map(|row| {
            format!(
                "| {} | {} | {} | {} | {} |",
                escape_cell(row.host),
                measure(row.current, "total"),
                measure(row.base, "total"),
                measure(row.current, "userspace"),
                measure(row.base, "userspace")
            )
        }));
        lines.push(String::new());
        lines
    }

    fn several_systems(&self) -> bool {
        self.inputs.perf.len() > 1
    }

    fn machine_name(&self, host: &str, system: &str) -> String {
        if self.several_systems() {
            return format!("{host} ({system})");
        }

        host.to_owned()
    }

    fn unmeasured(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for (system, measures) in &self.inputs.perf {
            let Some(skipped) = object(measures.get("skipped")) else {
                continue;
            };

            for (host, reason) in skipped {
                let reason = match reason.as_str() {
                    Some("absent") => "not on the base revision, so nothing to compare it with",
                    Some("no-vm") => "it does not boot in a VM on the base revision",
                    _ => "not measured",
                };
                lines.push(format!(
                    "- {}: {reason}.",
                    code(&self.machine_name(host, system))
                ));
            }
        }
        if lines.is_empty() {
            return lines;
        }

        let count = lines.len();
        let mut section = vec![
            format!(
                "{count} {} not measured:",
                plural(count, "machine is", "machines are")
            ),
            String::new(),
        ];
        section.extend(lines);
        section.push(String::new());
        section
    }

    fn performance(&self) -> Vec<String> {
        let unmeasured = self.unmeasured();
        let empty = Map::new();
        let machines: Vec<Machine> = self
            .inputs
            .perf
            .iter()
            .flat_map(|(system, measures)| {
                let rounds = match measures.get("rounds") {
                    Some(Value::Number(rounds)) => Some(rounds),
                    _ => None,
                };
                object(measures.get("hosts"))
                    .into_iter()
                    .flatten()
                    .filter_map(move |(host, sides)| {
                        let sides = sides.as_object()?;
                        Some((system, rounds, host, sides))
                    })
            })
            .map(|(system, rounds, host, sides)| Machine {
                name: self.machine_name(host, system),
                rounds,
                base: object(sides.get("base")).unwrap_or(&empty),
                current: object(sides.get("current")).unwrap_or(&empty),
            })
            .collect();
        if machines.is_empty() && unmeasured.is_empty() {
            return Vec::new();
        }

        let mut lines = vec!["### Performance against the base".to_owned(), String::new()];
        if machines.is_empty() {
            lines.extend(unmeasured);
            return lines;
        }

        let flags: Vec<String> = machines
            .iter()
            .flat_map(|machine| METRICS.iter().filter_map(|metric| machine.flag(metric)))
            .collect();
        let boots = machines
            .iter()
            .find_map(|machine| machine.rounds)
            .map_or_else(|| "several".to_owned(), Number::to_string);
        lines.push(format!(
            "{PERF_INTRO_START}{boots}{PERF_INTRO_MIDDLE}{boots}{PERF_INTRO_END}"
        ));
        lines.push(String::new());
        if flags.is_empty() {
            lines.push("No metric crossed its threshold.".to_owned());
        } else {
            lines.push(format!(
                "{} {}:",
                flags.len(),
                plural(
                    flags.len(),
                    "metric crossed its threshold",
                    "metrics crossed their threshold"
                )
            ));
            lines.push(String::new());
            lines.extend(flags);
        }
        lines.push(String::new());
        for machine in &machines {
            lines.extend(machine.table());
        }
        lines.extend(unmeasured);
        lines
    }

    fn screenshots(&self) -> Vec<String> {
        let mut shots: Vec<(&str, &str, Vec<String>)> = self
            .inputs
            .results
            .iter()
            .map(|result| {
                (
                    text(result, "check"),
                    text(result, "system"),
                    strings(result.get("screenshots")),
                )
            })
            .filter(|(_, _, screenshots)| !screenshots.is_empty())
            .collect();
        if shots.is_empty() {
            return Vec::new();
        }

        shots.sort_by(|left, right| (left.0, left.1).cmp(&(right.0, right.1)));
        let place = if self.inputs.run.is_empty() {
            "this run".to_owned()
        } else {
            format!("[this run]({}#artifacts)", self.inputs.run)
        };
        let mut lines = vec!["### Screenshots".to_owned(), String::new()];
        lines.extend(shots.iter().map(|(check, system, screenshots)| {
            let files: Vec<String> = screenshots.iter().map(|file| code(file)).collect();
            format!(
                "- {} ({system}): {} in the artifact {} of {place}.",
                code(check),
                files.join(", "),
                code(&format!("screenshot-{system}-{check}"))
            )
        }));
        lines.push(String::new());
        lines
    }

    fn not_planned(&self) -> Vec<String> {
        let run = &self.inputs.run;
        if self.inputs.plan == "failure" {
            let pointer = if run.is_empty() {
                "The `plan` job log names the file and line.".to_owned()
            } else {
                format!(
                    "The `plan` job log names the file and line: [open the run]({run}), then the `plan` job."
                )
            };
            return vec![
                "**The flake did not evaluate, so nothing was built or tested.**".to_owned(),
                String::new(),
                TO_FIX.to_owned(),
                String::new(),
                pointer,
            ];
        }

        let pointer = if run.is_empty() {
            "Open the full log to see why.".to_owned()
        } else {
            format!("[Open the full log]({run}) to see why.")
        };
        vec![
            "**CI could not plan this run, so nothing was built.**".to_owned(),
            String::new(),
            TO_FIX.to_owned(),
            String::new(),
            pointer,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Of {
    Min,
    Median,
    Whole,
}

#[derive(Debug, Clone, Copy)]
struct Metric {
    key: &'static str,
    name: &'static str,
    of: Of,
    higher_is_better: bool,
    relative: u32,
    absolute: f64,
}

const METRICS: [Metric; 8] = [
    Metric {
        key: "total",
        name: "Boot, total",
        of: Of::Min,
        higher_is_better: false,
        relative: 10,
        absolute: 0.5,
    },
    Metric {
        key: "userspace",
        name: "Boot, userspace",
        of: Of::Min,
        higher_is_better: false,
        relative: 10,
        absolute: 0.5,
    },
    Metric {
        key: "memory",
        name: "Idle RAM",
        of: Of::Min,
        higher_is_better: false,
        relative: 10,
        absolute: 32.0,
    },
    Metric {
        key: "closure",
        name: "Closure",
        of: Of::Whole,
        higher_is_better: false,
        relative: 5,
        absolute: 50.0 * MEBIBYTE,
    },
    Metric {
        key: "services",
        name: "Running services",
        of: Of::Median,
        higher_is_better: false,
        relative: 0,
        absolute: 1.0,
    },
    Metric {
        key: "cpu",
        name: "CPU, sysbench",
        of: Of::Median,
        higher_is_better: true,
        relative: 5,
        absolute: 100.0,
    },
    Metric {
        key: "memoryBandwidth",
        name: "Memory bandwidth, sysbench",
        of: Of::Median,
        higher_is_better: true,
        relative: 5,
        absolute: 1000.0,
    },
    Metric {
        key: "randomRead",
        name: "Disk, 4k random read",
        of: Of::Median,
        higher_is_better: true,
        relative: 10,
        absolute: 2000.0,
    },
];

#[derive(Debug, Clone)]
struct Measure {
    value: f64,
    text: String,
}

impl Measure {
    fn computed(value: f64) -> Self {
        Self {
            value,
            text: jq_number(value),
        }
    }
}

impl Metric {
    fn label(&self) -> String {
        let suffix = match self.of {
            Of::Min => " (best)",
            Of::Median => " (median)",
            Of::Whole => "",
        };
        format!("{}{suffix}", self.name)
    }

    fn measure(&self, side: &Map<String, Value>) -> Option<Measure> {
        let container = match self.of {
            Of::Whole => Some(side),
            Of::Min => object(side.get("min")),
            Of::Median => object(side.get("median")),
        };
        let Some(Value::Number(number)) = container?.get(self.key) else {
            return None;
        };

        Some(Measure {
            value: number.as_f64()?,
            text: number.to_string(),
        })
    }

    fn value(&self, measure: Option<&Measure>) -> String {
        let Some(measure) = measure else {
            return "n/a".to_owned();
        };

        let value = measure.value;
        match self.key {
            "total" | "userspace" => seconds(Some(value)),
            "memory" => format!("{} MiB", one_decimal(value)),
            "closure" => format!("{} MiB", one_decimal(value / MEBIBYTE)),
            "cpu" => format!("{} events/s", one_decimal(value)),
            "memoryBandwidth" => format!("{} MiB/s", one_decimal(value)),
            "randomRead" => format!("{} IOPS", jq_number(value.round())),
            _ => measure.text.clone(),
        }
    }

    fn limit(&self) -> String {
        if self.relative == 0 {
            return if self.higher_is_better {
                "any decrease"
            } else {
                "any increase"
            }
            .to_owned();
        }

        let sign = if self.higher_is_better { '-' } else { '+' };
        format!(
            "{sign}{} % and {sign}{}",
            self.relative,
            self.value(Some(&Measure::computed(self.absolute)))
        )
    }

    fn crossed(&self, base: f64, current: f64, change: f64) -> bool {
        let relative = f64::from(self.relative);
        if self.higher_is_better {
            return change < -relative && base - current >= self.absolute;
        }

        change > relative && current - base >= self.absolute
    }
}

fn change(base: Option<&Measure>, current: Option<&Measure>) -> Option<f64> {
    let (base, current) = (base?.value, current?.value);
    if base == 0.0 {
        return (current == 0.0).then_some(0.0);
    }

    Some((current - base) / base * 100.0)
}

fn percent(change: Option<f64>) -> String {
    let Some(change) = change else {
        return "n/a".to_owned();
    };

    let text = one_decimal(change);
    if text == "-0.0" {
        return "0.0 %".to_owned();
    }

    if text.starts_with('-') || text == "0.0" {
        return format!("{text} %");
    }

    format!("+{text} %")
}

struct Machine<'a> {
    name: String,
    rounds: Option<&'a Number>,
    base: &'a Map<String, Value>,
    current: &'a Map<String, Value>,
}

impl Machine<'_> {
    fn flag(&self, metric: &Metric) -> Option<String> {
        let base = metric.measure(self.base)?;
        let current = metric.measure(self.current)?;
        let change = change(Some(&base), Some(&current))?;
        if !metric.crossed(base.value, current.value, change) {
            return None;
        }

        let difference = metric.value(Some(&Measure::computed(current.value - base.value)));
        let difference = if difference.starts_with('-') {
            difference
        } else {
            format!("+{difference}")
        };
        Some(format!(
            "- {}: {} {}, {difference} (threshold: {}).",
            code(&self.name),
            metric.name,
            percent(Some(change)),
            metric.limit()
        ))
    }

    fn table(&self) -> Vec<String> {
        let mut lines = vec![
            format!("#### {}", self.name),
            String::new(),
            "| Metric | Base | This change | Change |".to_owned(),
            "|---|---|---|---|".to_owned(),
        ];
        lines.extend(METRICS.iter().map(|metric| {
            let base = metric.measure(self.base);
            let current = metric.measure(self.current);
            format!(
                "| {} | {} | {} | {} |",
                metric.label(),
                metric.value(base.as_ref()),
                metric.value(current.as_ref()),
                percent(change(base.as_ref(), current.as_ref()))
            )
        }));
        lines.push(String::new());
        lines
    }
}

#[must_use]
pub fn render(inputs: &Inputs, causes: &Causes) -> String {
    let report = Report { inputs, causes };
    let lines = match &inputs.planned {
        None => report.not_planned(),
        Some(planned) => report.planned(planned),
    };
    lines.iter().fold(String::new(), |mut out, line| {
        out.push_str(line);
        out.push('\n');
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_as_jq_prints_them() {
        let printed: Vec<String> = [
            0.0,
            -0.0,
            7.0,
            4.3,
            2_147_483_648.0,
            1e16,
            1.5e16,
            1.5e300,
            7e-5,
            0.0007,
            -2.5e17,
        ]
        .into_iter()
        .map(jq_number)
        .collect();
        assert_eq!(
            printed,
            [
                "0",
                "-0",
                "7",
                "4.3",
                "2147483648",
                "1e+16",
                "15000000000000000",
                "1.5e+300",
                "7e-05",
                "0.0007",
                "-2.5e+17"
            ]
        );
    }
}
