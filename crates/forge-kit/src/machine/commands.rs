use std::env;
use std::process::Command;
use std::time::Duration;

use super::Machine;
use crate::invocation::{Invocation, Output};
use crate::os::executable::find_in_path;
use crate::ports::commands::{Answer, Commands};
use crate::process::{self, Stderr};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

impl Commands for Machine {
    fn run(&self, invocation: &Invocation) -> Answer {
        let answer = answer(invocation);
        process::die_if_stopping();
        answer
    }
}

fn answer(invocation: &Invocation) -> Answer {
    let mut command = command(invocation);
    let timeout = invocation.timeout.unwrap_or(COMMAND_TIMEOUT);
    let failed = |message: &str| {
        if invocation.output == Output::Shown {
            eprint!("{message}");
        }
        Answer::failed(message)
    };
    let cannot_run = |error| failed(&format!("cannot run {}: {error}", invocation.program));

    match invocation.output {
        Output::Attached => match command.status() {
            Ok(status) => Answer {
                success: status.success(),
                code: status.code(),
                ..Answer::default()
            },
            Err(error) => cannot_run(error),
        },
        Output::Discarded => match process::status_within(&mut command, timeout) {
            Ok(Some(status)) => Answer {
                success: status.success(),
                code: status.code(),
                ..Answer::default()
            },
            Ok(None) => Answer::failed("timed out"),
            Err(error) => cannot_run(error),
        },
        Output::Captured | Output::Shown => {
            let limit = invocation.output_limit.unwrap_or(process::OUTPUT_LIMIT);
            let stderr = if invocation.output == Output::Shown {
                Stderr::Shown
            } else {
                Stderr::Kept
            };
            match process::run_capped(
                &mut command,
                invocation.input.as_deref().map(Vec::as_slice),
                timeout,
                limit,
                stderr,
            ) {
                Ok(Some(captured)) if captured.overflowed => {
                    failed(&format!("printed more than {limit} bytes"))
                }
                Ok(Some(captured)) => Answer {
                    success: captured.status.success(),
                    code: captured.status.code(),
                    stdout: String::from_utf8_lossy(&captured.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&captured.stderr).into_owned(),
                },
                Ok(None) => failed("timed out"),
                Err(error) => cannot_run(error),
            }
        }
    }
}

pub(super) fn command(invocation: &Invocation) -> Command {
    let Some(owner) = invocation.run_as else {
        let mut command = Command::new(&invocation.program);
        command
            .args(&invocation.args)
            .args(&invocation.os_args)
            .envs(invocation.env.iter().map(|(name, value)| (name, value)));
        return command;
    };

    let search_path = env::var("PATH").unwrap_or_default();
    let env_program = find_in_path("env", &search_path)
        .map_or_else(|| "env".to_owned(), |path| path.display().to_string());
    let mut command = Command::new("setpriv");
    command
        .arg(format!("--reuid={}", owner.uid))
        .arg(format!("--regid={}", owner.gid))
        .args(["--init-groups", "--reset-env"])
        .arg(env_program)
        .arg(format!("PATH={search_path}"))
        .args(
            invocation
                .env
                .iter()
                .map(|(name, value)| format!("{name}={value}")),
        )
        .arg(&invocation.program)
        .args(&invocation.args)
        .args(&invocation.os_args);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raised_output_limit_keeps_an_output_the_default_one_refuses() {
        let long = Invocation::new("head", &["-c", "1100000", "/dev/zero"]);

        let refused = Machine.run(&long);
        let kept = Machine.run(&long.clone().output_limit(2 * 1024 * 1024));

        assert!(!refused.success);
        assert_eq!(refused.stderr, "printed more than 1048576 bytes");
        assert!(kept.success);
        assert_eq!(kept.stdout.len(), 1_100_000);
    }
}
