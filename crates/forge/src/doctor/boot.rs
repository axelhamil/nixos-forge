use std::io;
use std::path::Path;

use super::report::{Report, Status};
use super::{Context, ROOT_HINT};
use crate::system::{boot_entries, generation_went_back};

const CHECK: &str = "Boot";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    if !context.root {
        return report.line(Status::Skip, CHECK, &format!("needs root, {ROOT_HINT}"));
    }

    let Some(newest) = boot_entries(context.system.commands)
        .into_iter()
        .max_by_key(|entry| entry.generation)
    else {
        return report.line(
            Status::Skip,
            CHECK,
            "no systemd-boot entries on this machine",
        );
    };

    let went_back = generation_went_back(
        context.system.files,
        &newest,
        Path::new("/nix/var/nix/profiles"),
        Path::new("/run/current-system"),
    );
    if !went_back {
        return report.line(
            Status::Ok,
            CHECK,
            "the newest generation started, or has tries left",
        );
    }

    let mut hint = "journalctl -b -1 shows why".to_owned();
    if context.config.auto_upgrade_state.is_some() {
        hint.push_str(", forge doctor --history lists the upgrades");
    }
    report.hinted(
        Status::Warn,
        CHECK,
        "the last update did not start, this machine went back to the previous version",
        &hint,
    )
}
