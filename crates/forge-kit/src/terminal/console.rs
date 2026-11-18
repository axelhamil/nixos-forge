use std::io::{self, Write};
use std::mem;
use std::os::fd::{AsFd, BorrowedFd};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::io::Errno;
use rustix::termios::{LocalModes, OptionalActions, Termios, isatty, tcgetattr, tcsetattr};
use zeroize::{Zeroize, Zeroizing};

use crate::interrupt::Flag;
use crate::ports::terminal::Terminal;

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
        let mut line = self.read_line()?;
        Ok(mem::take(&mut *line))
    }

    fn secret(&mut self, prompt: &str) -> io::Result<Zeroizing<String>> {
        self.ask(&format!("{prompt}: "))?;
        let answer = {
            let _echo_off = self
                .is_interactive()
                .then(|| EchoOff::new(self.input.as_fd()))
                .transpose()?;
            self.read_line()
        };
        writeln!(self.output)?;
        answer
    }

    fn is_interactive(&self) -> bool {
        isatty(self.input.as_fd())
    }
}

const LINE_CAPACITY: usize = 256;

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

    fn read_line(&self) -> io::Result<Zeroizing<String>> {
        let mut line = Zeroizing::new(Vec::with_capacity(LINE_CAPACITY));
        loop {
            self.wait_for_input()?;
            let mut byte = Zeroizing::new([0]);
            match rustix::io::read(self.input.as_fd(), &mut *byte) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(_) if *byte == *b"\n" => break,
                Ok(_) => push_without_copy(&mut line, byte.as_slice()),
                Err(Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let mut text = match String::from_utf8(mem::take(&mut *line)) {
            Ok(text) => Zeroizing::new(text),
            Err(error) => {
                let utf8 = error.utf8_error();
                error.into_bytes().zeroize();
                return Err(io::Error::new(io::ErrorKind::InvalidData, utf8));
            }
        };
        let (start, end) = trimmed_range(&text);
        text.truncate(end);
        text.drain(..start);
        Ok(text)
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

fn push_without_copy(line: &mut Zeroizing<Vec<u8>>, bytes: &[u8]) {
    if line.len() + bytes.len() > line.capacity() {
        let mut grown = Zeroizing::new(Vec::with_capacity(
            (line.capacity() * 2).max(line.len() + bytes.len()),
        ));
        grown.extend_from_slice(line);
        *line = grown;
    }
    line.extend_from_slice(bytes);
}

fn trimmed_range(text: &str) -> (usize, usize) {
    let kept = text.trim_end_matches([' ', '\t']);
    let end = kept.len();
    (end - kept.trim_start_matches([' ', '\t']).len(), end)
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
    fn a_secret_off_a_terminal_is_read_like_a_line_and_ends_its_line_like_read_s() {
        let mut console = console(" hunter2\t\nhunter2\n");

        assert_eq!(
            console.secret("Disk passphrase").unwrap().as_str(),
            "hunter2"
        );
        assert_eq!(console.secret("Same again").unwrap().as_str(), "hunter2");
        assert_eq!(console.output, b"\n\n");
    }

    #[test]
    fn a_secret_closed_before_the_newline_is_no_answer() {
        let error = console("hunter").secret("Disk passphrase").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn a_line_off_a_terminal_is_read_without_its_prompt_and_trimmed_like_read() {
        let mut console = console("  2 \t\n/dev/vda\n");

        assert_eq!(console.line("Disk to install on, 1 to 2: ").unwrap(), "2");
        assert_eq!(console.line("Type /dev/vda: ").unwrap(), "/dev/vda");
        assert_eq!(console.output, Vec::<u8>::new());
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
