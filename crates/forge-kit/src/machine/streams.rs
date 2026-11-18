use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Stdio;

use super::Machine;
use super::commands::command;
use crate::invocation::{Invocation, Output};
use crate::ports::streams::{Streamed, Streams};
use crate::process;

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

        let shown = invocation.output != Output::Discarded;
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
            let chunk = buffer.get(..read).unwrap_or_default();
            if shown {
                let _ = stdout.write_all(chunk).and_then(|()| stdout.flush());
            }
            if logged.is_ok() {
                logged = file.write_all(chunk);
            }
        }
        let succeeded = child.wait()?.success();
        process::die_if_stopping();
        Ok(Streamed { succeeded, logged })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

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
