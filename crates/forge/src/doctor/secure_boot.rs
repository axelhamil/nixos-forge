use std::io;
use std::path::Path;

use super::Context;
use super::report::{Report, Status};

const CHECK: &str = "Secure Boot";
const EFI_GLOBAL: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    if !context.config.secure_boot {
        return report.line(Status::Skip, CHECK, "secure-boot is not imported");
    }

    if !context.system.files.is_dir(Path::new("/sys/firmware/efi")) {
        return report.hinted(
            Status::Fail,
            CHECK,
            "not booted through UEFI",
            "boot the machine in UEFI mode, not legacy BIOS",
        );
    }

    if efi_variable(context, "SetupMode") == Some(1) {
        return report.hinted(
            Status::Fail,
            CHECK,
            "firmware in setup mode, keys not enrolled",
            "reboot: systemd-boot enrolls the keys from /boot/loader/keys/auto",
        );
    }
    if efi_variable(context, "SecureBoot") == Some(1) {
        return report.line(Status::Ok, CHECK, "enforced, keys enrolled");
    }
    report.hinted(
        Status::Fail,
        CHECK,
        "disabled in the firmware",
        "turn Secure Boot on in the firmware settings, keeping the enrolled keys",
    )
}

fn efi_variable(context: &Context, name: &str) -> Option<u8> {
    let path = format!("/sys/firmware/efi/efivars/{name}-{EFI_GLOBAL}");
    let bytes = context.system.files.read(Path::new(&path)).ok()?;
    bytes.get(4).copied()
}
