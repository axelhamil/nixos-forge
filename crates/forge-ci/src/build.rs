use std::env;
use std::ffi::OsString;
use std::fs::{self, File, Permissions};
use std::io::{self, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use forge::system::{Invocation, Machine, Streams};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::excerpt::excerpt;

const USAGE: &str = "usage: ci-build <system> <check> <out-dir> [--base-input <input> --base <flake-ref>]
  BASE_INPUT with BASE_SHA stands for --base-input $BASE_INPUT --base github:$GITHUB_REPOSITORY/$BASE_SHA";

const NO_REVISION: &str = "/0000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    system: String,
    check: String,
    out: PathBuf,
    base: Option<Base>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Base {
    input: String,
    flake: String,
}

#[derive(Debug, Default)]
struct Environment {
    base_input: Option<String>,
    base_sha: String,
    repository: String,
}

impl Environment {
    fn read() -> Self {
        Self {
            base_input: env::var("BASE_INPUT").ok(),
            base_sha: env::var("BASE_SHA").unwrap_or_default(),
            repository: env::var("GITHUB_REPOSITORY").unwrap_or_default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("cannot create {}: {source}", path.display())]
    Create { path: PathBuf, source: io::Error },
    #[error("cannot write {}: {source}", path.display())]
    Write { path: PathBuf, source: io::Error },
}

#[derive(Serialize)]
struct BuildResult<'a> {
    check: &'a str,
    system: &'a str,
    outcome: &'a str,
    excerpt: Vec<String>,
    #[serde(rename = "bootTime")]
    boot_time: Value,
    screenshots: Vec<String>,
}

#[derive(Default)]
struct Evidence {
    boot_time: Value,
    screenshots: Vec<String>,
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let Some(request) = parse(args, &Environment::read()) else {
        eprintln!("{USAGE}");
        return 2;
    };

    match build(&Machine, &request) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(error) => {
            eprintln!("ci-build: {error}");
            1
        }
    }
}

fn parse(args: &[OsString], environment: &Environment) -> Option<Request> {
    let args: Vec<&str> = args.iter().map(|arg| arg.to_str()).collect::<Option<_>>()?;
    let [system, check, out, options @ ..] = args.as_slice() else {
        return None;
    };

    let mut base = "";
    let mut base_input = "";
    let mut options = options.iter();
    while let Some(&option) = options.next() {
        let value = match option {
            "--base" | "--base-input" => *options.next()?,
            _ => return None,
        };
        if option == "--base" {
            base = value;
        } else {
            base_input = value;
        }
    }

    let from_environment;
    if base_input.is_empty()
        && let Some(input) = environment
            .base_input
            .as_deref()
            .filter(|input| !input.is_empty())
    {
        base_input = input;
        from_environment = format!("github:{}/{}", environment.repository, environment.base_sha);
        base = &from_environment;
    }

    if base.is_empty() != base_input.is_empty() {
        return None;
    }

    Some(Request {
        system: (*system).to_owned(),
        check: (*check).to_owned(),
        out: PathBuf::from(out),
        base: (!base.is_empty()).then(|| Base {
            input: base_input.to_owned(),
            flake: base.to_owned(),
        }),
    })
}

fn build(streams: &dyn Streams, request: &Request) -> Result<bool, Error> {
    let mut args = vec![
        "build".to_owned(),
        "--no-link".to_owned(),
        "--print-build-logs".to_owned(),
        "--print-out-paths".to_owned(),
    ];
    if let Some(base) = &request.base {
        if base.flake.ends_with(NO_REVISION) {
            println!(
                "{} has no base revision ({}), it runs against its locked {}",
                request.check, base.flake, base.input
            );
        } else {
            println!(
                "{} runs against {} as {}",
                request.check, base.flake, base.input
            );
            args.extend([
                "--override-input".to_owned(),
                base.input.clone(),
                base.flake.clone(),
            ]);
        }
    }
    args.push(format!(".#checks.{}.{}", request.system, request.check));

    let work = tempfile::tempdir().map_err(|source| Error::Create {
        path: env::temp_dir(),
        source,
    })?;
    fs::create_dir_all(&request.out).map_err(|source| Error::Create {
        path: request.out.clone(),
        source,
    })?;

    let log = work.path().join("log");
    let out_paths = work.path().join("out-paths");
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let invocation = Invocation::new("nix", &args).stdout_to(&out_paths);
    let passed = match streams.stream(&invocation, &log) {
        Ok(streamed) => streamed.succeeded,
        Err(error) => {
            eprintln!("ci-build: cannot run nix: {error}");
            false
        }
    };

    let (excerpt, evidence) = if passed {
        let evidence = keep_evidence(&out_paths, request).unwrap_or_else(|_| {
            eprintln!(
                "ci-build: the boot time, measures or screenshots of {} could not be kept, the check still passed",
                request.check
            );
            Evidence::default()
        });
        (Vec::new(), evidence)
    } else {
        let excerpt = File::open(&log)
            .and_then(|file| excerpt(BufReader::new(file)))
            .unwrap_or_default();
        (excerpt, Evidence::default())
    };

    let result = BuildResult {
        check: &request.check,
        system: &request.system,
        outcome: if passed { "passed" } else { "failed" },
        excerpt,
        boot_time: evidence.boot_time,
        screenshots: evidence.screenshots,
    };
    let path = request
        .out
        .join(format!("{}-{}.json", request.system, request.check));
    let text = serde_json::to_string_pretty(&result).map_err(io::Error::from);
    text.and_then(|text| fs::write(&path, text + "\n"))
        .map_err(|source| Error::Write { path, source })?;
    Ok(passed)
}

fn keep_evidence(out_paths: &Path, request: &Request) -> io::Result<Evidence> {
    let paths = fs::read_to_string(out_paths)?;
    let path = Path::new(paths.lines().next().unwrap_or_default());
    if path.as_os_str().is_empty() || !path.is_dir() {
        return Ok(Evidence::default());
    }

    let mut evidence = Evidence::default();
    let boot_time = path.join("boot-time.json");
    if boot_time.is_file() {
        evidence.boot_time = boot_time_of(&fs::read_to_string(boot_time)?)?;
    }

    let perf = path.join("perf.json");
    if perf.is_file() {
        let dir = request.out.join("perf");
        fs::create_dir_all(&dir)?;
        install(&perf, &dir.join(format!("{}.json", request.system)))?;
    }

    let screenshots = screenshots(path)?;
    if screenshots.is_empty() {
        return Ok(evidence);
    }
    let dir = request.out.join("screenshots");
    fs::create_dir_all(&dir)?;
    for name in &screenshots {
        let target = format!("{}-{}-{name}", request.system, request.check);
        install(&path.join(name), &dir.join(target))?;
    }
    evidence.screenshots = screenshots;
    Ok(evidence)
}

fn boot_time_of(text: &str) -> io::Result<Value> {
    let time: Value = serde_json::from_str(text)?;
    let time = match time {
        Value::Object(time) => time,
        Value::Null => Map::new(),
        _ => return Err(io::Error::other("the boot time is not an object")),
    };
    let field = |name: &str| time.get(name).cloned().unwrap_or(Value::Null);
    let mut kept = Map::new();
    kept.insert("total".to_owned(), field("total"));
    kept.insert("userspace".to_owned(), field("userspace"));
    Ok(Value::Object(kept))
}

fn screenshots(path: &Path) -> io::Result<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().is_some_and(|extension| extension == "png")
        })
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    Ok(names)
}

fn install(source: &Path, target: &Path) -> io::Result<()> {
    fs::copy(source, target)?;
    fs::set_permissions(target, Permissions::from_mode(0o644))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_arguments_win_over_the_environment() {
        let environment = Environment {
            base_input: Some("migration-base".to_owned()),
            base_sha: "0123abc".to_owned(),
            repository: "owner/repo".to_owned(),
        };
        let request = parse(
            &args(&[
                "x",
                "c",
                "out",
                "--base-input",
                "other",
                "--base",
                "path:/b",
            ]),
            &environment,
        )
        .unwrap();
        assert_eq!(
            request.base,
            Some(Base {
                input: "other".to_owned(),
                flake: "path:/b".to_owned(),
            })
        );
    }

    #[test]
    fn an_empty_base_and_input_stand_for_no_base() {
        let request = parse(
            &args(&["x", "c", "out", "--base", "", "--base-input", ""]),
            &Environment::default(),
        )
        .unwrap();
        assert_eq!(request.base, None);
    }

    #[test]
    fn the_boot_time_keeps_its_total_and_userspace_only() {
        assert_eq!(
            boot_time_of(r#"{"total":7.04,"kernel":1}"#).unwrap(),
            serde_json::json!({"total": 7.04, "userspace": null})
        );
        assert!(boot_time_of("[1]").is_err());
        assert!(boot_time_of("not json").is_err());
    }
}
