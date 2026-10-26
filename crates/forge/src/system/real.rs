use std::env;
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{
    Accounts, Answer, Clock, Commands, Directory, Files, Invocation, Output, Owner, Streamed,
    Streams,
};
use crate::enroll::tpm2::Host;
use crate::install::Storage;
use crate::{plugins, process};

pub struct Machine;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const TPM_DEVICE: &str = "/dev/tpmrm0";
const UNLOCK_KEY_BYTES: usize = 48;

impl Commands for Machine {
    fn run(&self, invocation: &Invocation) -> Answer {
        let answer = answer(invocation);
        if invocation.output == Output::Shown {
            eprint!("{}", answer.stderr);
        }
        answer
    }
}

fn answer(invocation: &Invocation) -> Answer {
    let mut command = command(invocation);
    let timeout = invocation.timeout.unwrap_or(COMMAND_TIMEOUT);
    let cannot_run = |error| Answer::failed(&format!("cannot run {}: {error}", invocation.program));

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
            match process::run_with_input(&mut command, invocation.input.as_deref(), timeout) {
                Ok(Some(captured)) if captured.overflowed => Answer::failed(&format!(
                    "printed more than {} bytes",
                    process::OUTPUT_LIMIT
                )),
                Ok(Some(captured)) => Answer {
                    success: captured.status.success(),
                    code: captured.status.code(),
                    stdout: String::from_utf8_lossy(&captured.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&captured.stderr).into_owned(),
                },
                Ok(None) => Answer::failed("timed out"),
                Err(error) => cannot_run(error),
            }
        }
    }
}

impl Streams for Machine {
    fn stream(&self, invocation: &Invocation, log: &Path) -> io::Result<Streamed> {
        let mut file = File::create(log)?;
        let (mut reader, writer) = io::pipe()?;
        let mut command = command(invocation);
        command
            .stdin(Stdio::null())
            .stdout(writer.try_clone()?)
            .stderr(writer);
        let mut child = command.spawn()?;
        drop(command);

        let mut stdout = io::stdout();
        let mut logged = Ok(());
        let mut buffer = [0; 8192];
        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let chunk = &buffer[..read];
            let _ = stdout.write_all(chunk).and_then(|()| stdout.flush());
            if logged.is_ok() {
                logged = file.write_all(chunk);
            }
        }
        Ok(Streamed {
            succeeded: child.wait()?.success(),
            logged,
        })
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

impl Clock for Machine {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }

    fn zone(&self) -> TimeZone {
        TimeZone::system()
    }
}

impl Accounts for Machine {
    fn directory(&self) -> Directory {
        let read = |path| fs::read_to_string(path).unwrap_or_default();
        Directory::parse(&read("/etc/passwd"), &read("/etc/group"))
    }
}

impl Files for Machine {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_link(&self, path: &Path) -> Option<PathBuf> {
        fs::read_link(path).ok()
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
        let gid = self
            .directory()
            .primary_group(uid)
            .unwrap_or(metadata.gid());
        Some(Owner { uid, gid })
    }
}

impl Storage for Machine {
    fn is_block_device(&self, path: &Path) -> bool {
        fs::metadata(path).is_ok_and(|metadata| metadata.file_type().is_block_device())
    }

    fn link(&self, target: &Path, link: &Path) -> io::Result<()> {
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_if_exists(link)?;
        symlink(target, link)
    }

    fn install_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        fs::create_dir_all(path)?;
        fs::set_permissions(path, Permissions::from_mode(mode))
    }

    fn install_file(&self, path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
        write_with_mode(path, bytes, mode)
    }

    fn remove(&self, path: &Path) {
        let _ = remove_if_exists(path);
    }
}

impl Host for Machine {
    fn has_tpm(&self) -> bool {
        fs::metadata(TPM_DEVICE).is_ok_and(|metadata| metadata.file_type().is_char_device())
    }

    fn write_unlock_key(&self, path: &Path) -> io::Result<()> {
        let mut random = [0; UNLOCK_KEY_BYTES];
        File::open("/dev/urandom")?.read_exact(&mut random)?;
        let key = random.iter().fold(String::new(), |mut key, byte| {
            let _ = write!(key, "{byte:02x}");
            key
        });
        write_with_mode(path, key.as_bytes(), 0o600)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        remove_if_exists(path)
    }
}

fn write_with_mode(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)?;
    file.set_permissions(Permissions::from_mode(mode))?;
    file.write_all(bytes)
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_reaches_the_log_with_both_outputs_and_its_status() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("rebuild.log");

        let streamed = Machine
            .stream(
                &Invocation::new("sh", &["-c", "echo out; echo err >&2; exit 3"]),
                &log,
            )
            .unwrap();

        assert!(!streamed.succeeded);
        assert!(streamed.logged.is_ok());
        assert_eq!(fs::read_to_string(&log).unwrap(), "out\nerr\n");
    }

    #[test]
    fn a_log_that_cannot_be_written_lets_the_rebuild_finish() {
        let streamed = Machine
            .stream(
                &Invocation::new(
                    "sh",
                    &[
                        "-c",
                        "i=0; while [ $i -lt 20000 ]; do echo line $i; i=$((i+1)); done",
                    ],
                ),
                Path::new("/dev/full"),
            )
            .unwrap();

        assert!(streamed.succeeded);
        assert_eq!(
            streamed.logged.map_err(|error| error.kind()),
            Err(io::ErrorKind::StorageFull)
        );
    }
}
