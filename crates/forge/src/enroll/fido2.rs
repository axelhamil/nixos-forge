use std::io::Write;

use super::exit_code;
use crate::system::{Commands, Invocation};

pub const USAGE: &str =
    "Usage: forge enroll fido2 <LUKS device>, this host has no disk feature to default to";

pub fn run(
    device: Option<&str>,
    default_device: Option<&str>,
    commands: &dyn Commands,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let Some(device) = device
        .or(default_device)
        .filter(|device| !device.is_empty())
    else {
        let _ = writeln!(err, "{USAGE}");
        return 1;
    };

    let enrolled = commands
        .run(&Invocation::new("systemd-cryptenroll", &["--fido2-device=auto", device]).attached());
    if !enrolled.success {
        return exit_code(enrolled.code);
    }

    let _ = writeln!(
        out,
        "Enrolled a security key on {device}, the passphrase stays as a fallback"
    );
    0
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::system::{Answer, Output};

    struct Cryptenroll {
        answer: Answer,
        runs: RefCell<Vec<Invocation>>,
    }

    impl Cryptenroll {
        fn answering(answer: Answer) -> Self {
            Self {
                answer,
                runs: RefCell::default(),
            }
        }

        fn command_lines(&self) -> Vec<String> {
            self.runs
                .borrow()
                .iter()
                .map(Invocation::command_line)
                .collect()
        }
    }

    impl Commands for Cryptenroll {
        fn run(&self, invocation: &Invocation) -> Answer {
            self.runs.borrow_mut().push(invocation.clone());
            self.answer.clone()
        }
    }

    fn enroll(
        device: Option<&str>,
        default_device: Option<&str>,
        commands: &Cryptenroll,
    ) -> (u8, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(device, default_device, commands, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn the_device_given_wins_over_the_disk_of_the_host() {
        let commands = Cryptenroll::answering(Answer::exited(0));

        let (code, out, _) = enroll(
            Some("/dev/vdb"),
            Some("/dev/disk/by-partlabel/disk-main-luks"),
            &commands,
        );

        assert_eq!(code, 0);
        assert_eq!(
            commands.command_lines(),
            ["systemd-cryptenroll --fido2-device=auto /dev/vdb"]
        );
        assert_eq!(commands.runs.borrow()[0].output, Output::Attached);
        assert_eq!(
            out,
            "Enrolled a security key on /dev/vdb, the passphrase stays as a fallback\n"
        );
    }

    #[test]
    fn without_a_device_the_disk_of_the_host_is_enrolled() {
        let commands = Cryptenroll::answering(Answer::exited(0));

        let (code, _, _) = enroll(
            None,
            Some("/dev/disk/by-partlabel/disk-main-luks"),
            &commands,
        );

        assert_eq!(code, 0);
        assert_eq!(
            commands.command_lines(),
            ["systemd-cryptenroll --fido2-device=auto /dev/disk/by-partlabel/disk-main-luks"]
        );
    }

    #[test]
    fn without_any_device_the_usage_is_printed_and_nothing_runs() {
        let commands = Cryptenroll::answering(Answer::exited(0));

        let (code, out, err) = enroll(None, None, &commands);

        assert_eq!(code, 1);
        assert_eq!(out, "");
        assert_eq!(err, format!("{USAGE}\n"));
        assert!(commands.command_lines().is_empty());
    }

    #[test]
    fn a_failed_enrollment_passes_its_exit_code_and_claims_nothing() {
        let commands = Cryptenroll::answering(Answer::exited(4));

        let (code, out, _) = enroll(Some("/dev/vdb"), None, &commands);

        assert_eq!(code, 4);
        assert_eq!(out, "");
    }
}
