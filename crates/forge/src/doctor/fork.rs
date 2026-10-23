use std::io;
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;

use super::Context;
use super::report::{CHECK_WIDTH, Report, Status};
use crate::plugins::{self, CONTRACT, CONTRACT_VARIABLE, DESCRIBE_TIMEOUT, Plugin};
use crate::process;

pub const DOCTOR_FLAG: &str = "--forge-doctor";
pub const ROOT_VARIABLE: &str = "FORGE_DOCTOR_ROOT";
pub const FORK_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
struct Line {
    status: Status,
    check: String,
    detail: String,
    #[serde(default)]
    hint: String,
}

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let forks = &context.system.forks;
    for plugin in plugins::list(&forks.dirs, &forks.builtins) {
        let joins = plugins::describe(&plugin, DESCRIBE_TIMEOUT)
            .is_ok_and(|description| description.doctor);
        if joins {
            run(context, &plugin, report)?;
        }
    }
    Ok(())
}

fn run(context: &Context, plugin: &Plugin, report: &mut Report) -> io::Result<()> {
    let name = format!("forge-{}", plugin.name);
    let timeout = context.system.forks.timeout;
    let captured = process::run_within(
        Command::new(&plugin.path)
            .arg(DOCTOR_FLAG)
            .env(CONTRACT_VARIABLE, CONTRACT.to_string())
            .env(ROOT_VARIABLE, if context.root { "1" } else { "0" }),
        timeout,
    );

    let captured = match captured {
        Ok(Some(captured)) => captured,
        Ok(None) => {
            return report.line(
                Status::Fail,
                &name,
                &format!(
                    "`{name} {DOCTOR_FLAG}` did not answer within {}s and was stopped",
                    timeout.as_secs_f32()
                ),
            );
        }
        Err(error) => {
            return report.line(Status::Fail, &name, &format!("cannot run {name}: {error}"));
        }
    };

    if captured.overflowed {
        return report.line(
            Status::Fail,
            &name,
            &format!(
                "`{name} {DOCTOR_FLAG}` printed more than {} bytes, its output was cut",
                process::OUTPUT_LIMIT
            ),
        );
    }

    let stdout = String::from_utf8_lossy(&captured.stdout);
    let mut reported = 0;
    let mut invalid = None;
    for text in stdout.lines().filter(|text| !text.trim().is_empty()) {
        match parse(text) {
            Ok(line) => {
                report.hinted(line.status, &line.check, &line.detail, &line.hint)?;
                reported += 1;
            }
            Err(reason) => {
                invalid = Some(reason);
                break;
            }
        }
    }

    let problem = if let Some(reason) = invalid {
        Some(format!("printed an invalid line: {reason}"))
    } else if !captured.status.success() {
        Some(format!(
            "`{name} {DOCTOR_FLAG}` failed ({})",
            captured.status
        ))
    } else if reported == 0 {
        Some(format!("`{name} {DOCTOR_FLAG}` printed no check"))
    } else {
        None
    };
    match problem {
        Some(problem) => report.line(Status::Fail, &name, &problem),
        None => Ok(()),
    }
}

fn parse(text: &str) -> Result<Line, String> {
    let line: Line = serde_json::from_str(text).map_err(|error| error.to_string())?;
    if line.check.is_empty() || line.check.chars().count() > CHECK_WIDTH {
        return Err(format!(
            "check `{}` must have 1 to {CHECK_WIDTH} characters",
            line.check
        ));
    }
    Ok(line)
}
