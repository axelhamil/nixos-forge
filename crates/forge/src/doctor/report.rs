use std::fmt;
use std::io::{self, Write};

use forge_contracts::plugin::{CHECK_WIDTH, DoctorStatus};

const INDENT: &str = "                     ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Warn,
    Fail,
    Skip,
}

impl From<DoctorStatus> for Status {
    fn from(status: DoctorStatus) -> Self {
        match status {
            DoctorStatus::Ok => Self::Ok,
            DoctorStatus::Warn => Self::Warn,
            DoctorStatus::Fail => Self::Fail,
            DoctorStatus::Skip => Self::Skip,
        }
    }
}

impl Status {
    fn color(self) -> &'static str {
        match self {
            Self::Ok => "\x1b[32m",
            Self::Warn => "\x1b[33m",
            Self::Fail => "\x1b[31m",
            Self::Skip => "\x1b[2m",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
        };
        formatter.pad(name)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub failures: usize,
    pub warnings: usize,
}

pub struct Report<'w> {
    out: &'w mut dyn Write,
    colors: bool,
    totals: Totals,
}

impl<'w> Report<'w> {
    pub fn new(out: &'w mut dyn Write, colors: bool) -> Self {
        Self {
            out,
            colors,
            totals: Totals::default(),
        }
    }

    pub fn line(&mut self, status: Status, check: &str, detail: &str) -> io::Result<()> {
        self.hinted(status, check, detail, "")
    }

    pub fn hinted(
        &mut self,
        status: Status,
        check: &str,
        detail: &str,
        hint: &str,
    ) -> io::Result<()> {
        let (color, reset) = if self.colors {
            (status.color(), "\x1b[0m")
        } else {
            ("", "")
        };
        writeln!(
            self.out,
            "{color}[{status:>4}]{reset} {check:<CHECK_WIDTH$} {detail}"
        )?;
        if !hint.is_empty() {
            self.detail(hint)?;
        }

        match status {
            Status::Fail => self.totals.failures += 1,
            Status::Warn => self.totals.warnings += 1,
            Status::Ok | Status::Skip => {}
        }
        Ok(())
    }

    pub fn detail(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.out, "{INDENT}{text}")
    }

    pub fn finish(self) -> io::Result<Totals> {
        writeln!(self.out)?;
        writeln!(
            self.out,
            "failed: {}, warnings: {}",
            self.totals.failures, self.totals.warnings
        )?;
        Ok(self.totals)
    }
}
