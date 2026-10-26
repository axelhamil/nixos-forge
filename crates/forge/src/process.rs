use std::io::{self, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use rustix::io::Errno;
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};

pub const OUTPUT_LIMIT: u64 = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const GRACE: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct Captured {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub overflowed: bool,
}

struct Output {
    bytes: Vec<u8>,
    overflowed: bool,
}

type Reader = Receiver<io::Result<Output>>;

pub fn run_within(command: &mut Command, timeout: Duration) -> io::Result<Option<Captured>> {
    run_with_input(command, None, timeout)
}

pub fn run_with_input(
    command: &mut Command,
    input: Option<&[u8]>,
    timeout: Duration,
) -> io::Result<Option<Captured>> {
    let deadline = Instant::now() + timeout;
    let stdin = if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    let mut child = command
        .process_group(0)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    write_input(child.stdin.take(), input);

    let stdout = read_capped(child.stdout.take());
    let stderr = read_capped(child.stderr.take());

    let Some(status) = finish(&mut child, deadline)? else {
        return Ok(None);
    };

    let (Some(stdout), Some(stderr)) = (receive(&stdout, deadline)?, receive(&stderr, deadline)?)
    else {
        return Ok(None);
    };
    Ok(Some(Captured {
        status,
        overflowed: stdout.overflowed || stderr.overflowed,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    }))
}

pub fn status_within(command: &mut Command, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    let mut child = command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    finish(&mut child, deadline)
}

fn finish(child: &mut Child, deadline: Instant) -> io::Result<Option<ExitStatus>> {
    let group = Pid::from_child(child);
    let exited = exits_before(group, deadline)?;
    if !exited {
        signal_group(group, Signal::TERM)?;
        exits_before(group, Instant::now() + GRACE)?;
    }
    signal_group(group, Signal::KILL)?;
    let status = child.wait()?;
    Ok(exited.then_some(status))
}

fn write_input(stdin: Option<ChildStdin>, input: Option<&[u8]>) {
    let (Some(mut stdin), Some(input)) = (stdin, input) else {
        return;
    };
    let input = input.to_vec();
    thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
}

trait Pipe: Read + Send + 'static {}
impl Pipe for ChildStdout {}
impl Pipe for ChildStderr {}

fn read_capped(pipe: Option<impl Pipe>) -> Reader {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(read_up_to_limit(pipe));
    });
    receiver
}

fn read_up_to_limit(pipe: Option<impl Pipe>) -> io::Result<Output> {
    let mut bytes = Vec::new();
    let Some(mut pipe) = pipe else {
        return Ok(Output {
            bytes,
            overflowed: false,
        });
    };
    pipe.by_ref().take(OUTPUT_LIMIT).read_to_end(&mut bytes)?;
    let overflowed = pipe.read(&mut [0; 1])? > 0;
    Ok(Output { bytes, overflowed })
}

fn receive(reader: &Reader, deadline: Instant) -> io::Result<Option<Output>> {
    let left = deadline.saturating_duration_since(Instant::now());
    match reader.recv_timeout(left) {
        Ok(output) => output.map(Some),
        Err(RecvTimeoutError::Timeout) => Ok(None),
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other(
            "the reader of a child's output stopped without an answer",
        )),
    }
}

fn exits_before(child: Pid, deadline: Instant) -> io::Result<bool> {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOWAIT | WaitIdOptions::NOHANG;
    loop {
        if waitid(WaitId::Pid(child), options)?.is_some() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn signal_group(group: Pid, signal: Signal) -> io::Result<()> {
    match kill_process_group(group, signal) {
        Ok(()) | Err(Errno::SRCH) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(script: &str, timeout: Duration) -> Option<Captured> {
        run_within(Command::new("sh").args(["-c", script]), timeout).unwrap()
    }

    #[test]
    fn a_child_that_answers_in_time_is_captured() {
        let captured = run("echo out; echo err >&2; exit 3", Duration::from_secs(5)).unwrap();

        assert_eq!(captured.status.code(), Some(3));
        assert_eq!(captured.stdout, b"out\n");
        assert_eq!(captured.stderr, b"err\n");
        assert!(!captured.overflowed);
    }

    #[test]
    fn a_timeout_stops_the_whole_process_group() {
        let started = Instant::now();

        let captured = run("sleep 30 & sleep 30; wait", Duration::from_millis(200));

        assert!(captured.is_none());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_timeout_lets_the_group_clean_up_before_it_stops() {
        let dir = tempfile::tempdir().unwrap();
        let cleaned = dir.path().join("cleaned");
        let script = format!(
            "trap 'echo cleaned > {}; exit 1' TERM; sleep 30 & wait",
            cleaned.display()
        );

        let captured = run(&script, Duration::from_millis(300));

        assert!(captured.is_none());
        assert!(cleaned.exists());
    }

    #[test]
    fn a_group_that_ignores_the_stop_request_is_killed_after_the_grace() {
        let started = Instant::now();

        let captured = run("trap '' TERM; sleep 30", Duration::from_millis(200));

        assert!(captured.is_none());
        assert!(started.elapsed() < GRACE + Duration::from_secs(3));
    }

    #[test]
    fn a_background_child_holding_the_output_does_not_outlive_the_timeout() {
        let started = Instant::now();

        let captured = run("sleep 30 & exit 0", Duration::from_millis(200));

        assert!(captured.is_some());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn the_input_reaches_the_child_on_its_stdin() {
        let captured = run_with_input(
            Command::new("sh").args(["-c", "read -r line; echo \"got $line\""]),
            Some(b"secret\n"),
            Duration::from_secs(5),
        )
        .unwrap()
        .unwrap();

        assert_eq!(captured.stdout, b"got secret\n");
    }

    #[test]
    fn an_endless_output_is_cut_at_the_limit() {
        let captured = run("yes", Duration::from_secs(10)).unwrap();

        assert!(captured.overflowed);
        assert_eq!(captured.stdout.len(), 1024 * 1024);
    }
}
