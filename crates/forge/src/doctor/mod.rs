mod boot;
mod boot_setup;
mod config;
mod fork;
mod format;
mod history;
mod impermanence;
mod lost_files;
mod luks;
mod mountinfo;
pub mod real;
mod report;
mod secure_boot;
mod signers;
mod snapshots;
mod space;
mod system;
mod units;
mod updates;
mod upgrades;

use std::io::{self, Write};

use jiff::Timestamp;
use jiff::tz::TimeZone;

pub use config::{CONFIG_FILE, CONFIG_VARIABLE, Config, ConfigError};
pub use fork::{DOCTOR_FLAG, FORK_TIMEOUT, ROOT_VARIABLE};
pub use report::{Status, Totals};
pub use system::{
    Answer, Clock, Commands, Files, Forks, Host, Invocation, Mount, Owner, Space, System,
};

pub const ROOT_HINT: &str = "run sudo forge doctor";

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub all: bool,
    pub colors: bool,
}

struct Context<'a> {
    config: &'a Config,
    system: &'a System<'a>,
    root: bool,
    now: Timestamp,
    zone: TimeZone,
    all: bool,
}

impl Context<'_> {
    fn short_date(&self, timestamp: Timestamp) -> String {
        format::short_date(timestamp, &self.zone)
    }
}

impl<'a> Context<'a> {
    fn new(config: &'a Config, system: &'a System<'a>, all: bool) -> Self {
        Self {
            config,
            system,
            root: system.host.uid() == 0,
            now: system.clock.now(),
            zone: system.clock.zone(),
            all,
        }
    }
}

pub fn history(config: &Config, system: &System, out: &mut dyn Write) -> io::Result<()> {
    history::show(&Context::new(config, system, false), out)
}

pub fn run(
    config: &Config,
    system: &System,
    options: Options,
    out: &mut dyn Write,
) -> io::Result<Totals> {
    let context = Context::new(config, system, options.all);

    if !context.root {
        writeln!(
            out,
            "Running without root: the checks that need it are skipped. Run sudo forge doctor for all of them."
        )?;
        writeln!(out)?;
    }

    let mut report = report::Report::new(out, options.colors);
    secure_boot::check(&context, &mut report)?;
    luks::check(&context, &mut report)?;
    snapshots::check(&context, &mut report)?;
    space::check(&context, &mut report)?;
    units::check(&context, &mut report)?;
    updates::check(&context, &mut report)?;
    upgrades::check(&context, &mut report)?;
    signers::check(&context, &mut report)?;
    boot::check(&context, &mut report)?;
    impermanence::check(&context, &mut report)?;
    fork::check(&context, &mut report)?;
    report.finish()
}
