use std::io::{self, Write};
use std::os::fd::{AsFd, BorrowedFd};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::io::Errno;
use rustix::termios::{LocalModes, OptionalActions, Termios, isatty, tcgetattr, tcsetattr};

use super::interrupt::Flag;

pub trait Terminal {
    fn line(&mut self, prompt: &str) -> io::Result<String>;
    fn secret(&mut self, prompt: &str) -> io::Result<String>;
    fn is_interactive(&self) -> bool;
}

pub struct Console<I, O> {
    input: I,
    output: O,
    interrupt: Option<Flag>,
}

impl Console<io::Stdin, io::Stderr> {
    #[must_use]
    pub fn stdio(interrupt: Option<Flag>) -> Self {
        Self::new(io::stdin(), io::stderr(), interrupt)
    }
}

impl<I: AsFd, O: Write> Console<I, O> {
    pub fn new(input: I, output: O, interrupt: Option<Flag>) -> Self {
        Self {
            input,
            output,
            interrupt,
        }
    }
}

impl<I: AsFd, O: Write> Terminal for Console<I, O> {
    fn line(&mut self, prompt: &str) -> io::Result<String> {
        self.ask(prompt)?;
        self.read_line()
    }

    fn secret(&mut self, prompt: &str) -> io::Result<String> {
        if !self.is_interactive() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("cannot read {prompt}: stdin is not a terminal"),
            ));
        }

        self.ask(&format!("{prompt}: "))?;
        let answer = {
            let _echo_off = EchoOff::new(self.input.as_fd())?;
            self.read_line()
        };
        writeln!(self.output)?;
        answer
    }

    fn is_interactive(&self) -> bool {
        isatty(self.input.as_fd())
    }
}

const POLL_INTERVAL: Timespec = Timespec {
    tv_sec: 0,
    tv_nsec: 100_000_000,
};

impl<I: AsFd, O: Write> Console<I, O> {
    fn ask(&mut self, prompt: &str) -> io::Result<()> {
        if !self.is_interactive() {
            return Ok(());
        }
        write!(self.output, "{prompt}")?;
        self.output.flush()
    }

    fn read_line(&self) -> io::Result<String> {
        let mut line = Vec::new();
        loop {
            self.wait_for_input()?;
            let mut byte = [0];
            match rustix::io::read(self.input.as_fd(), &mut byte) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(_) if byte[0] == b'\n' => break,
                Ok(_) => line.push(byte[0]),
                Err(Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let line = String::from_utf8(line)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(line.trim_matches([' ', '\t']).to_owned())
    }

    fn wait_for_input(&self) -> io::Result<()> {
        let Some(interrupt) = &self.interrupt else {
            return Ok(());
        };

        loop {
            if let Some(signal) = interrupt.raised() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    format!("stopped by signal {signal}"),
                ));
            }
            let mut fds = [PollFd::new(&self.input, PollFlags::IN)];
            match poll(&mut fds, Some(&POLL_INTERVAL)) {
                Ok(0) | Err(Errno::INTR) => {}
                Ok(_) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }
}

struct EchoOff<'fd> {
    fd: BorrowedFd<'fd>,
    saved: Termios,
}

impl<'fd> EchoOff<'fd> {
    fn new(fd: BorrowedFd<'fd>) -> io::Result<Self> {
        let saved = tcgetattr(fd)?;
        let mut silent = saved.clone();
        silent.local_modes.remove(LocalModes::ECHO);
        tcsetattr(fd, OptionalActions::Now, &silent)?;
        Ok(Self { fd, saved })
    }
}

impl Drop for EchoOff<'_> {
    fn drop(&mut self) {
        let _ = tcsetattr(self.fd, OptionalActions::Now, &self.saved);
    }
}

#[cfg(test)]
mod tests {
    use std::io::PipeReader;

    use super::*;

    fn console(typed: &str) -> Console<PipeReader, Vec<u8>> {
        let (reader, mut writer) = io::pipe().unwrap();
        writer.write_all(typed.as_bytes()).unwrap();
        Console::new(reader, Vec::new(), None)
    }

    #[test]
    fn piped_input_is_not_interactive() {
        assert!(!console("").is_interactive());
    }

    #[test]
    fn a_secret_is_refused_without_a_terminal_and_nothing_is_read() {
        let mut console = console("hunter2\n");

        let error = console.secret("Disk passphrase").unwrap_err();

        assert_eq!(
            error.to_string(),
            "cannot read Disk passphrase: stdin is not a terminal"
        );
        assert_eq!(console.line("Next").unwrap(), "hunter2");
    }

    #[test]
    fn a_line_off_a_terminal_is_read_without_its_prompt_and_trimmed_like_read() {
        let mut console = console("  2 \t\n/dev/vda\n");

        assert_eq!(console.line("Disk to install on, 1 to 2: ").unwrap(), "2");
        assert_eq!(console.line("Type /dev/vda: ").unwrap(), "/dev/vda");
        assert!(console.output.is_empty());
    }

    #[test]
    fn input_closed_before_the_newline_is_no_answer() {
        let mut console = console("/dev/vd");

        let error = console.line("Type /dev/vda: ").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn an_interrupt_stops_a_prompt_waiting_for_input() {
        let (reader, _still_open) = io::pipe().unwrap();
        let mut console = Console::new(reader, Vec::new(), Some(Flag::raised_by(2)));

        let error = console.line("Disk to install on, 1 to 2: ").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    }
}
