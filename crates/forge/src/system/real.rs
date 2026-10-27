use std::env;
use std::fmt::Write as _;
use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{
    DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt, symlink,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{
    Accounts, Answer, Clock, Commands, Directory, Files, Invocation, Output, Owner, Sizes, Storage,
    Streamed, Streams, space,
};
use crate::enroll::tpm2::Host;
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
            let limit = invocation.output_limit.unwrap_or(process::OUTPUT_LIMIT);
            match process::run_capped(&mut command, invocation.input.as_deref(), timeout, limit) {
                Ok(Some(captured)) if captured.overflowed => {
                    Answer::failed(&format!("printed more than {limit} bytes"))
                }
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
        let stdout = match &invocation.stdout_file {
            Some(path) => Stdio::from(File::create(path)?),
            None => Stdio::from(writer.try_clone()?),
        };
        let mut command = command(invocation);
        command.stdin(Stdio::null()).stdout(stdout).stderr(writer);
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

pub(crate) fn find_in_path(program: &str, search_path: &str) -> Option<PathBuf> {
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

    fn exists(&self, path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok()
    }

    fn link(&self, target: &Path, link: &Path) -> io::Result<()> {
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_if_exists(link)?;
        symlink(target, link)
    }

    fn create_new_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        DirBuilder::new().mode(mode).create(path)
    }

    fn install_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        fs::create_dir_all(path)?;
        fs::set_permissions(path, Permissions::from_mode(mode))
    }

    fn install_file(&self, path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
        write_with_mode(path, bytes, mode)
    }

    fn create_dirs(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        fs::write(path, bytes)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove(&self, path: &Path) {
        let _ = remove_if_exists(path);
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }
}

impl Sizes for Machine {
    fn available(&self, path: &Path) -> io::Result<u64> {
        space(path).map(|space| space.avail)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        fs::metadata(path).map(|metadata| metadata.len())
    }

    fn tree_size(&self, path: &Path) -> u64 {
        walkdir::WalkDir::new(path)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.metadata().ok())
            .map(|metadata| metadata.len())
            .sum()
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
    fn a_new_dir_gets_its_mode_and_an_existing_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let made = dir.path().join("deploy-key");

        Machine.create_new_dir(&made, 0o700).unwrap();

        let permissions = fs::metadata(&made).unwrap().permissions();
        assert_eq!(permissions.mode() & 0o777, 0o700);
        assert_eq!(
            Machine
                .create_new_dir(&made, 0o700)
                .map_err(|error| error.kind()),
            Err(io::ErrorKind::AlreadyExists)
        );
    }

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
    fn a_stream_sends_its_stdout_to_a_file_when_asked_and_logs_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("build.log");
        let out_paths = dir.path().join("out-paths");

        let streamed = Machine
            .stream(
                &Invocation::new("sh", &["-c", "echo out; echo err >&2"]).stdout_to(&out_paths),
                &log,
            )
            .unwrap();

        assert!(streamed.succeeded);
        assert_eq!(fs::read_to_string(&log).unwrap(), "err\n");
        assert_eq!(fs::read_to_string(&out_paths).unwrap(), "out\n");
    }

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
