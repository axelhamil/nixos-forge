use std::io;

use super::report::{Report, Status};
use super::{Context, Invocation, ROOT_HINT};

const CHECK: &str = "LUKS unlock";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let Some(device) = &context.config.luks_device else {
        return report.line(Status::Skip, CHECK, "needs disk and secure-boot");
    };

    if !context.root {
        return report.line(Status::Skip, CHECK, &format!("needs root, {ROOT_HINT}"));
    }

    let device = device.display().to_string();
    let answer = context
        .system
        .commands
        .run(&Invocation::new("systemd-cryptenroll", &[&device]));
    if !answer.success {
        let output = format!("{}{}", answer.stdout, answer.stderr);
        return report.hinted(
            Status::Fail,
            CHECK,
            &format!("cannot read the keyslots of {device}"),
            output.trim_end(),
        );
    }

    let types: Vec<&str> = answer
        .stdout
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().nth(1))
        .collect();

    if !types
        .iter()
        .any(|kind| matches!(*kind, "password" | "recovery"))
    {
        return report.hinted(
            Status::Fail,
            CHECK,
            "no passphrase keyslot, nothing to fall back on if the TPM2 refuses",
            &format!("add one now: sudo systemd-cryptenroll --password {device}"),
        );
    }
    if !types.contains(&"tpm2") {
        return report.hinted(
            Status::Warn,
            CHECK,
            "no TPM2 token, the passphrase is asked at every boot",
            "tpm2-enroll adds one once Secure Boot is enforced: journalctl -b -u tpm2-enroll",
        );
    }
    report.line(
        Status::Ok,
        CHECK,
        "TPM2 token, passphrase kept as a fallback",
    )
}
