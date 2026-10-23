use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{Answer, Clock, Commands, Files, Host, Invocation, Mount, Owner, Space, mountinfo};
use crate::{plugins, process};

pub struct Machine;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

impl Commands for Machine {
    fn run(&self, invocation: &Invocation) -> Answer {
        let mut command = command(invocation);
        let timeout = invocation.timeout.unwrap_or(COMMAND_TIMEOUT);
        let cannot_run =
            |error| Answer::failed(&format!("cannot run {}: {error}", invocation.program));

        if invocation.discard_output {
            return match process::status_within(&mut command, timeout) {
                Ok(Some(status)) => Answer {
                    success: status.success(),
                    ..Answer::default()
                },
                Ok(None) => Answer::failed("timed out"),
                Err(error) => cannot_run(error),
            };
        }

        match process::run_within(&mut command, timeout) {
            Ok(Some(captured)) if captured.overflowed => Answer::failed(&format!(
                "printed more than {} bytes",
                process::OUTPUT_LIMIT
            )),
            Ok(Some(captured)) => Answer {
                success: captured.status.success(),
                stdout: String::from_utf8_lossy(&captured.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&captured.stderr).into_owned(),
            },
            Ok(None) => Answer::failed("timed out"),
            Err(error) => cannot_run(error),
        }
    }
}

fn command(invocation: &Invocation) -> Command {
    let Some(owner) = invocation.run_as else {
        let mut command = Command::new(&invocation.program);
        command
            .args(&invocation.args)
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
        .args(&invocation.args);
    command
}

fn find_in_path(program: &str, search_path: &str) -> Option<PathBuf> {
    env::split_paths(search_path)
        .map(|dir| dir.join(program))
        .find(|candidate| plugins::is_executable(candidate))
}

impl Files for Machine {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn list(&self, dir: &Path) -> Vec<String> {
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect()
    }

    fn canonicalize(&self, path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn owner(&self, path: &Path) -> Option<Owner> {
        let metadata = fs::metadata(path).ok()?;
        let uid = metadata.uid();
        let gid = primary_group(uid).unwrap_or(metadata.gid());
        Some(Owner { uid, gid })
    }
}

fn primary_group(uid: u32) -> Option<u32> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|entry| {
        let fields: Vec<&str> = entry.split(':').collect();
        let entry_uid: u32 = fields.get(2)?.parse().ok()?;
        if entry_uid != uid {
            return None;
        }
        fields.get(3)?.parse().ok()
    })
}

impl Host for Machine {
    fn uid(&self) -> u32 {
        rustix::process::getuid().as_raw()
    }

    fn sudo_user(&self) -> Option<String> {
        env::var("SUDO_USER").ok().filter(|user| !user.is_empty())
    }

    fn space(&self, path: &Path) -> io::Result<Space> {
        let stats = rustix::fs::statvfs(path)?;
        Ok(Space {
            size: stats.f_blocks.saturating_mul(stats.f_frsize),
            avail: stats.f_bavail.saturating_mul(stats.f_frsize),
        })
    }

    fn mounts(&self) -> io::Result<Vec<Mount>> {
        fs::read_to_string("/proc/self/mountinfo").map(|text| mountinfo::parse(&text))
    }
}

impl Clock for Machine {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }

    fn zone(&self) -> TimeZone {
        TimeZone::system()
    }
}
