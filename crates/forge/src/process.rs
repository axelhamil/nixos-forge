use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use rustix::io::Errno;
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};

pub const OUTPUT_LIMIT: u64 = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

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
    let deadline = Instant::now() + timeout;
    let mut child = command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

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
    stop_group(group)?;
    let status = child.wait()?;
    Ok(exited.then_some(status))
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

fn stop_group(group: Pid) -> io::Result<()> {
    match kill_process_group(group, Signal::KILL) {
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
    fn a_background_child_holding_the_output_does_not_outlive_the_timeout() {
        let started = Instant::now();

        let captured = run("sleep 30 & exit 0", Duration::from_millis(200));

        assert!(captured.is_some());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn an_endless_output_is_cut_at_the_limit() {
        let captured = run("yes", Duration::from_secs(10)).unwrap();

        assert!(captured.overflowed);
        assert_eq!(captured.stdout.len(), 1024 * 1024);
    }
}
