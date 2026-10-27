use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use forge::system::{Commands, Invocation, Machine};

use crate::model::{REPRODUCIBILITY_HEADING, REPRODUCIBILITY_INTRO, Verdict};

const USAGE: &str = "usage: reproducibility [<host>...]";
const LOG_LINES: usize = 20;
const DIFF_LINES: usize = 200;
const EVALUATION_TIMEOUT: Duration = Duration::from_mins(90);
const DIFFOSCOPE_TIMEOUT: Duration = Duration::from_mins(240);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub succeeded: bool,
    pub stdout: String,
}

pub trait Builds {
    fn build(&self, invocation: &Invocation, log: &Path) -> io::Result<Built>;
}

impl Builds for Machine {
    fn build(&self, invocation: &Invocation, log: &Path) -> io::Result<Built> {
        let mut child = Command::new(&invocation.program)
            .args(&invocation.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(File::create(log)?)
            .spawn()?;
        let mut stdout = Vec::new();
        if let Some(mut pipe) = child.stdout.take() {
            pipe.read_to_end(&mut stdout)?;
        }
        Ok(Built {
            succeeded: child.wait()?.success(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ReproducibilityError {
    #[error("reproducibility: {0} failed")]
    Hosts(String),
    #[error("reproducibility: {0} printed no list of hosts")]
    HostsInvalid(String),
    #[error("reproducibility: cannot {action}: {error}")]
    Io {
        action: &'static str,
        error: io::Error,
    },
}

fn io_error(action: &'static str) -> impl FnOnce(io::Error) -> ReproducibilityError {
    move |error| ReproducibilityError::Io { action, error }
}

fn hosts(commands: &dyn Commands) -> Result<Vec<String>, ReproducibilityError> {
    let invocation = Invocation::new(
        "nix",
        &[
            "eval",
            "--impure",
            "--json",
            ".#ci.hosts",
            "--apply",
            "hosts: hosts.${builtins.currentSystem}",
        ],
    )
    .timeout(EVALUATION_TIMEOUT)
    .loud();
    let answer = commands.run(&invocation);
    if !answer.success {
        return Err(ReproducibilityError::Hosts(invocation.command_line()));
    }
    serde_json::from_str(&answer.stdout)
        .map_err(|_| ReproducibilityError::HostsInvalid(invocation.command_line()))
}

fn lines_of(file: &Path) -> Vec<String> {
    let text = fs::read(file)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    text.lines().map(|line| format!("{line}\n")).collect()
}

fn tail(log: &Path, lines: usize) -> String {
    let all = lines_of(log);
    all[all.len().saturating_sub(lines)..].concat()
}

fn head(file: &Path, lines: usize) -> String {
    let all = lines_of(file);
    all[..all.len().min(lines)].concat()
}

fn remove(path: &Path) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    let _ = if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
}

struct Checker<'a> {
    commands: &'a dyn Commands,
    builds: &'a dyn Builds,
    work: PathBuf,
}

enum Checked {
    Verdict(Verdict),
    Failed(Verdict, String),
    Differs(String),
}

impl Checker<'_> {
    fn check(&self, host: &str) -> Result<Checked, ReproducibilityError> {
        let installable = format!(".#nixosConfigurations.{host}.config.system.build.toplevel");
        let build_log = self.work.join("build.log");
        let built = self
            .builds
            .build(
                &Invocation::new(
                    "nix",
                    &["build", "--no-link", "--print-out-paths", &installable],
                ),
                &build_log,
            )
            .map_err(io_error("run nix build"))?;
        if !built.succeeded {
            return Ok(Checked::Failed(
                Verdict::NotBuilt,
                tail(&build_log, LOG_LINES),
            ));
        }

        let path = built.stdout.trim_end_matches('\n').to_owned();
        let check = PathBuf::from(format!("{path}.check"));
        remove(&check);
        if fs::symlink_metadata(&check).is_ok() {
            return Ok(Checked::Failed(Verdict::LeftOver(path), String::new()));
        }

        let rebuild_log = self.work.join("rebuild.log");
        let rebuilt = self
            .builds
            .build(
                &Invocation::new(
                    "nix",
                    &[
                        "build",
                        "--no-link",
                        "--rebuild",
                        "--keep-failed",
                        &installable,
                    ],
                ),
                &rebuild_log,
            )
            .map_err(io_error("run nix build --rebuild"))?;
        if rebuilt.succeeded {
            return Ok(Checked::Verdict(Verdict::Identical));
        }

        if fs::symlink_metadata(&check).is_err() {
            return Ok(Checked::Failed(
                Verdict::NotFinished,
                tail(&rebuild_log, LOG_LINES),
            ));
        }

        let diff = self.work.join(format!("{host}.diff"));
        let diff_arg = diff.display().to_string();
        let check_arg = check.display().to_string();
        let _ = self.commands.run(
            &Invocation::new(
                "diffoscope",
                &[
                    "--text",
                    &diff_arg,
                    "--exclude-directory-metadata=yes",
                    &path,
                    &check_arg,
                ],
            )
            .timeout(DIFFOSCOPE_TIMEOUT)
            .loud(),
        );
        Ok(Checked::Differs(head(&diff, DIFF_LINES)))
    }
}

fn differs(host: &str, diff: &str) -> String {
    format!(
        "{}\n\n<details>\n<summary>diffoscope of {host}, first {DIFF_LINES} lines</summary>\n\n```\n{diff}```\n</details>\n\n",
        Verdict::Differs.line(host)
    )
}

fn compare(
    commands: &dyn Commands,
    builds: &dyn Builds,
    hosts: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<bool, ReproducibilityError> {
    let work = tempfile::tempdir().map_err(io_error("create a temporary directory"))?;
    let checker = Checker {
        commands,
        builds,
        work: work.path().to_owned(),
    };

    let _ = write!(
        out,
        "{REPRODUCIBILITY_HEADING}\n\n{REPRODUCIBILITY_INTRO}\n\n"
    );
    let mut ok = true;
    for host in hosts {
        match checker.check(host)? {
            Checked::Verdict(verdict) => {
                let _ = writeln!(out, "{}", verdict.line(host));
            }
            Checked::Failed(verdict, log) => {
                let _ = writeln!(out, "{}", verdict.line(host));
                let _ = write!(err, "{log}");
                ok = false;
            }
            Checked::Differs(diff) => {
                let _ = write!(out, "{}", differs(host, &diff));
            }
        }
        let _ = out.flush();
    }
    Ok(ok)
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let first = args.first().and_then(|arg| arg.to_str());
    if matches!(first, Some("-h" | "--help")) {
        println!("{USAGE}");
        return 0;
    }
    if first.is_some_and(|arg| arg.starts_with('-')) {
        eprintln!("{USAGE}");
        return 2;
    }

    let hosts = if args.is_empty() {
        match hosts(&Machine) {
            Ok(hosts) => hosts,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        }
    } else {
        args.iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    };

    match compare(
        &Machine,
        &Machine,
        &hosts,
        &mut io::stdout(),
        &mut io::stderr(),
    ) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}
