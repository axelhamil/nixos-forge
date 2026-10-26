use std::io;

use super::Context;
use super::report::{Report, Status};
use crate::system::efi::{self, SecureBoot};

const CHECK: &str = "Secure Boot";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    if !context.config.secure_boot {
        return report.line(Status::Skip, CHECK, "secure-boot is not imported");
    }

    match efi::secure_boot(context.system.files) {
        SecureBoot::NoUefi => report.hinted(
            Status::Fail,
            CHECK,
            "not booted through UEFI",
            "boot the machine in UEFI mode, not legacy BIOS",
        ),
        SecureBoot::SetupMode => report.hinted(
            Status::Fail,
            CHECK,
            "firmware in setup mode, keys not enrolled",
            "reboot: systemd-boot enrolls the keys from /boot/loader/keys/auto",
        ),
        SecureBoot::Enforced => report.line(Status::Ok, CHECK, "enforced, keys enrolled"),
        SecureBoot::Disabled => report.hinted(
            Status::Fail,
            CHECK,
            "disabled in the firmware",
            "turn Secure Boot on in the firmware settings, keeping the enrolled keys",
        ),
    }
}
