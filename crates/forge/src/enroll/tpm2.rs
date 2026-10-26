use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use super::exit_code;
use crate::system::efi::{self, SecureBoot};
use crate::system::interrupt::Flag;
use crate::system::luks::Header;
use crate::system::{Answer, Commands, Files, Invocation, Machine};

pub const TEMPORARY_SLOT: u32 = 31;

const TPM_DEVICE: &str = "/dev/tpmrm0";
const TPM2_TOKEN: &str = "systemd-tpm2";
const UNLOCK_KEY_BYTES: usize = 48;
const STOPPED: u8 = 143;

pub struct Settings<'a> {
    pub device: &'a str,
    pub volume_key_description: &'a str,
    pub pcrs: &'a str,
    pub runtime_dir: &'a Path,
}

pub trait Host {
    fn has_tpm(&self) -> bool;
    fn write_unlock_key(&self, path: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
}

pub struct Ports<'a> {
    pub commands: &'a dyn Commands,
    pub files: &'a dyn Files,
    pub host: &'a dyn Host,
    pub interrupt: &'a Flag,
}

struct Session {
    volume_key: String,
    unlock_key: PathBuf,
    temporary_key_added: bool,
}

pub fn run(settings: &Settings, ports: &Ports, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let device = settings.device;
    let mut header = match read_header(ports.commands, device, err) {
        Ok(header) => header,
        Err(code) => return code,
    };

    if is_leftover(&header) {
        let wiped = wipe_temporary_slot(ports.commands, device);
        if !wiped.success {
            return failure(&wiped, ports.interrupt);
        }
        let _ = writeln!(
            out,
            "Wiped the temporary keyslot {TEMPORARY_SLOT} left by an interrupted enrollment"
        );
        header = match read_header(ports.commands, device, err) {
            Ok(header) => header,
            Err(code) => return code,
        };
    }
    if ports.interrupt.raised().is_some() {
        return STOPPED;
    }

    let search = ports.commands.run(
        &Invocation::new(
            "keyctl",
            &["search", "@u", "user", settings.volume_key_description],
        )
        .loud(),
    );
    if !search.success {
        let _ = writeln!(
            out,
            "No volume key of {device} linked at this boot, nothing to enroll"
        );
        return 0;
    }

    let mut session = Session {
        volume_key: search.line().to_owned(),
        unlock_key: settings.runtime_dir.join("unlock-key"),
        temporary_key_added: false,
    };
    let code = enroll(settings, ports, &header, &mut session, out, err);

    if !clean_up(&session, device, ports, out, err) {
        let _ = writeln!(out, "Cleanup failed, check the keyslots of {device}");
        return 1;
    }
    code
}

fn read_header(commands: &dyn Commands, device: &str, err: &mut dyn Write) -> Result<Header, u8> {
    Header::read(commands, device).map_err(|error| {
        let _ = writeln!(err, "forge enroll tpm2: {error}");
        1
    })
}

fn is_leftover(header: &Header) -> bool {
    header.keyslots().contains(&TEMPORARY_SLOT)
        && header.keyslots().len() > 1
        && !header.slot_has_token(TEMPORARY_SLOT)
}

fn wipe_temporary_slot(commands: &dyn Commands, device: &str) -> Answer {
    let wipe = format!("--wipe-slot={TEMPORARY_SLOT}");
    commands.run(&Invocation::new("systemd-cryptenroll", &[&wipe, device]).attached())
}

fn failure(answer: &Answer, interrupt: &Flag) -> u8 {
    if interrupt.raised().is_some() {
        return STOPPED;
    }
    exit_code(answer.code)
}

fn enroll(
    settings: &Settings,
    ports: &Ports,
    header: &Header,
    session: &mut Session,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let device = settings.device;

    if header.has_token(TPM2_TOKEN) {
        let _ = writeln!(out, "{device} already has a TPM2 token, nothing to enroll");
        return 0;
    }
    if !ports.host.has_tpm() {
        let _ = writeln!(
            out,
            "No TPM2 on this machine, the passphrase stays the only way to unlock {device}"
        );
        return 0;
    }
    if efi::secure_boot(ports.files) != SecureBoot::Enforced {
        let _ = writeln!(
            out,
            "Secure Boot is not enforced yet, TPM2 enrollment of {device} waits for a boot in user mode"
        );
        return 0;
    }

    if let Err(error) = ports.host.write_unlock_key(&session.unlock_key) {
        let _ = writeln!(
            err,
            "forge enroll tpm2: cannot write {}: {error}",
            session.unlock_key.display()
        );
        return 1;
    }
    if ports.interrupt.raised().is_some() {
        return STOPPED;
    }

    let keyring = format!("%user:{}", settings.volume_key_description);
    let slot = TEMPORARY_SLOT.to_string();
    let unlock_key = session.unlock_key.display().to_string();
    let added = ports.commands.run(
        &Invocation::new(
            "cryptsetup",
            &[
                "luksAddKey",
                "--batch-mode",
                "--volume-key-keyring",
                &keyring,
                "--new-key-slot",
                &slot,
                "--pbkdf",
                "pbkdf2",
                "--pbkdf-force-iterations",
                "1000",
                device,
                &unlock_key,
            ],
        )
        .attached(),
    );
    if !added.success {
        return failure(&added, ports.interrupt);
    }
    session.temporary_key_added = true;
    if ports.interrupt.raised().is_some() {
        return STOPPED;
    }

    let unlock_key_file = format!("--unlock-key-file={unlock_key}");
    let pcrs = format!("--tpm2-pcrs={}", settings.pcrs);
    let enrolled = ports.commands.run(
        &Invocation::new(
            "systemd-cryptenroll",
            &[&unlock_key_file, "--tpm2-device=auto", &pcrs, device],
        )
        .attached(),
    );
    if !enrolled.success {
        return failure(&enrolled, ports.interrupt);
    }
    if ports.interrupt.raised().is_some() {
        return STOPPED;
    }

    let _ = writeln!(
        out,
        "Enrolled a TPM2 token on {device} bound to PCRs {}, the passphrase stays as a fallback",
        settings.pcrs
    );
    0
}

fn clean_up(
    session: &Session,
    device: &str,
    ports: &Ports,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> bool {
    let mut cleaned = ports
        .commands
        .run(&Invocation::new("keyctl", &["invalidate", &session.volume_key]).attached())
        .success;

    if session.temporary_key_added {
        if wipe_temporary_slot(ports.commands, device).success {
            let _ = writeln!(out, "Removed the temporary keyslot {TEMPORARY_SLOT}");
        } else {
            cleaned = false;
        }
    }

    if let Err(error) = ports.host.remove_file(&session.unlock_key) {
        let _ = writeln!(
            err,
            "forge enroll tpm2: cannot remove {}: {error}",
            session.unlock_key.display()
        );
        cleaned = false;
    }
    cleaned
}

impl Host for Machine {
    fn has_tpm(&self) -> bool {
        fs::metadata(TPM_DEVICE).is_ok_and(|metadata| metadata.file_type().is_char_device())
    }

    fn write_unlock_key(&self, path: &Path) -> io::Result<()> {
        let mut random = [0; UNLOCK_KEY_BYTES];
        File::open("/dev/urandom")?.read_exact(&mut random)?;
        let key = random.iter().fold(String::new(), |mut key, byte| {
            let _ = write!(key, "{byte:02x}");
            key
        });
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?
            .write_all(key.as_bytes())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        match fs::remove_file(path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use signal_hook::consts::SIGTERM;

    use super::*;
    use crate::system::Owner;

    const DEVICE: &str = "/dev/disk/by-partlabel/disk-main-luks";
    const DESCRIPTION: &str = "tpm2-enroll-cryptroot";
    const PCRS: &str = "7+15:sha256=0000";
    const RUNTIME_DIR: &str = "/run/tpm2-enroll";
    const UNLOCK_KEY: &str = "/run/tpm2-enroll/unlock-key";
    const KEY_ID: &str = "812345678";

    const PASSPHRASE_ONLY: &str = r#"{"keyslots": {"0": {}}, "tokens": {}}"#;
    const LEFTOVER_SLOT_31: &str = r#"{"keyslots": {"0": {}, "31": {}}, "tokens": {}}"#;
    const TPM2_ON_SLOT_1: &str = r#"{"keyslots": {"0": {}, "1": {}},
        "tokens": {"0": {"type": "systemd-tpm2", "keyslots": ["1"]}}}"#;

    struct Shell {
        headers: RefCell<Vec<&'static str>>,
        answers: Vec<(String, Answer)>,
        raise_on: Option<(String, Flag)>,
        runs: RefCell<Vec<String>>,
    }

    impl Shell {
        fn with_header(header: &'static str) -> Self {
            Self {
                headers: RefCell::new(vec![header]),
                answers: vec![(
                    format!("keyctl search @u user {DESCRIPTION}"),
                    Answer::ok(&format!("{KEY_ID}\n")),
                )],
                raise_on: None,
                runs: RefCell::default(),
            }
        }

        fn then_header(self, header: &'static str) -> Self {
            self.headers.borrow_mut().push(header);
            self
        }

        fn answer(mut self, command: &str, answer: Answer) -> Self {
            self.answers.insert(0, (command.to_owned(), answer));
            self
        }

        fn raise_during(mut self, command: &str, flag: &Flag) -> Self {
            self.raise_on = Some((command.to_owned(), flag.clone()));
            self
        }

        fn ran(&self, command: &str) -> bool {
            self.runs
                .borrow()
                .iter()
                .any(|run| run.starts_with(command))
        }
    }

    impl Commands for Shell {
        fn run(&self, invocation: &Invocation) -> Answer {
            let line = invocation.command_line();
            self.runs.borrow_mut().push(line.clone());

            if let Some((command, flag)) = &self.raise_on
                && line.starts_with(command.as_str())
            {
                flag.raise(SIGTERM);
            }
            if line.starts_with("cryptsetup luksDump") {
                let mut headers = self.headers.borrow_mut();
                let header = if headers.len() > 1 {
                    headers.remove(0)
                } else {
                    headers[0]
                };
                return Answer::ok(header);
            }
            self.answers
                .iter()
                .find(|(command, _)| line.starts_with(command.as_str()))
                .map_or_else(|| Answer::exited(0), |(_, answer)| answer.clone())
        }
    }

    struct Efivars {
        secure_boot: u8,
    }

    impl Files for Efivars {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            let name = path.file_name().unwrap().to_string_lossy();
            let value = if name.starts_with("SecureBoot-") {
                self.secure_boot
            } else {
                0
            };
            Ok(vec![7, 0, 0, 0, value])
        }

        fn read_link(&self, _: &Path) -> Option<PathBuf> {
            None
        }

        fn is_dir(&self, path: &Path) -> bool {
            path == Path::new("/sys/firmware/efi")
        }

        fn list(&self, _: &Path) -> Vec<String> {
            Vec::new()
        }

        fn canonicalize(&self, path: &Path) -> PathBuf {
            path.to_path_buf()
        }

        fn owner(&self, _: &Path) -> Option<Owner> {
            None
        }
    }

    struct Tpm {
        present: bool,
        removal_fails: bool,
        written: RefCell<Vec<PathBuf>>,
        removed: RefCell<Vec<PathBuf>>,
    }

    impl Tpm {
        fn present() -> Self {
            Self {
                present: true,
                removal_fails: false,
                written: RefCell::default(),
                removed: RefCell::default(),
            }
        }
    }

    impl Host for Tpm {
        fn has_tpm(&self) -> bool {
            self.present
        }

        fn write_unlock_key(&self, path: &Path) -> io::Result<()> {
            self.written.borrow_mut().push(path.to_path_buf());
            Ok(())
        }

        fn remove_file(&self, path: &Path) -> io::Result<()> {
            self.removed.borrow_mut().push(path.to_path_buf());
            if self.removal_fails {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(())
        }
    }

    struct Run {
        code: u8,
        out: String,
    }

    fn enroll_with(shell: &Shell, efivars: &Efivars, tpm: &Tpm, interrupt: &Flag) -> Run {
        let settings = Settings {
            device: DEVICE,
            volume_key_description: DESCRIPTION,
            pcrs: PCRS,
            runtime_dir: Path::new(RUNTIME_DIR),
        };
        let ports = Ports {
            commands: shell,
            files: efivars,
            host: tpm,
            interrupt,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&settings, &ports, &mut out, &mut err);
        Run {
            code,
            out: String::from_utf8(out).unwrap(),
        }
    }

    fn enroll(shell: &Shell) -> Run {
        enroll_with(
            shell,
            &Efivars { secure_boot: 1 },
            &Tpm::present(),
            &Flag::default(),
        )
    }

    fn wipe() -> String {
        format!("systemd-cryptenroll --wipe-slot=31 {DEVICE}")
    }

    #[test]
    fn a_leftover_slot_31_is_wiped_at_the_next_start_before_anything_else() {
        let shell = Shell::with_header(LEFTOVER_SLOT_31)
            .then_header(PASSPHRASE_ONLY)
            .answer("keyctl search", Answer::exited(1));

        let run = enroll(&shell);

        assert_eq!(run.code, 0);
        assert_eq!(
            shell.runs.borrow()[..3],
            [
                format!("cryptsetup luksDump --dump-json-metadata {DEVICE}"),
                wipe(),
                format!("cryptsetup luksDump --dump-json-metadata {DEVICE}"),
            ]
        );
        assert_eq!(
            run.out,
            format!(
                "Wiped the temporary keyslot 31 left by an interrupted enrollment\nNo volume key of {DEVICE} linked at this boot, nothing to enroll\n"
            )
        );
    }

    #[test]
    fn a_slot_31_that_is_the_only_keyslot_or_opened_by_a_token_is_kept() {
        let only = r#"{"keyslots": {"31": {}}, "tokens": {}}"#;
        let tokened = r#"{"keyslots": {"0": {}, "31": {}},
            "tokens": {"0": {"type": "systemd-fido2", "keyslots": ["31"]}}}"#;

        for header in [only, tokened] {
            let shell = Shell::with_header(header).answer("keyctl search", Answer::exited(1));

            enroll(&shell);

            assert!(!shell.ran(&wipe()), "{header}");
        }
    }

    #[test]
    fn a_failed_wipe_of_the_leftover_stops_with_its_exit_code() {
        let shell = Shell::with_header(LEFTOVER_SLOT_31).answer(&wipe(), Answer::exited(5));

        let run = enroll(&shell);

        assert_eq!(run.code, 5);
        assert!(!shell.ran("keyctl"));
    }

    #[test]
    fn without_a_linked_volume_key_nothing_is_enrolled_or_cleaned() {
        let shell = Shell::with_header(PASSPHRASE_ONLY).answer("keyctl search", Answer::exited(1));

        let run = enroll(&shell);

        assert_eq!(run.code, 0);
        assert_eq!(
            run.out,
            format!("No volume key of {DEVICE} linked at this boot, nothing to enroll\n")
        );
        assert!(!shell.ran("keyctl invalidate"));
    }

    #[test]
    fn an_existing_tpm2_token_ends_the_run_and_invalidates_the_key() {
        let shell = Shell::with_header(TPM2_ON_SLOT_1);

        let run = enroll(&shell);

        assert_eq!(run.code, 0);
        assert_eq!(
            run.out,
            format!("{DEVICE} already has a TPM2 token, nothing to enroll\n")
        );
        assert!(shell.ran(&format!("keyctl invalidate {KEY_ID}")));
        assert!(!shell.ran("cryptsetup luksAddKey"));
    }

    #[test]
    fn a_machine_without_a_tpm_keeps_the_passphrase_alone() {
        let shell = Shell::with_header(PASSPHRASE_ONLY);
        let tpm = Tpm {
            present: false,
            ..Tpm::present()
        };

        let run = enroll_with(&shell, &Efivars { secure_boot: 1 }, &tpm, &Flag::default());

        assert_eq!(run.code, 0);
        assert_eq!(
            run.out,
            format!(
                "No TPM2 on this machine, the passphrase stays the only way to unlock {DEVICE}\n"
            )
        );
        assert!(shell.ran("keyctl invalidate"));
        assert!(tpm.written.borrow().is_empty());
    }

    #[test]
    fn enrollment_waits_for_secure_boot_to_be_enforced() {
        let shell = Shell::with_header(PASSPHRASE_ONLY);

        let run = enroll_with(
            &shell,
            &Efivars { secure_boot: 0 },
            &Tpm::present(),
            &Flag::default(),
        );

        assert_eq!(run.code, 0);
        assert_eq!(
            run.out,
            format!(
                "Secure Boot is not enforced yet, TPM2 enrollment of {DEVICE} waits for a boot in user mode\n"
            )
        );
        assert!(!shell.ran("cryptsetup luksAddKey"));
    }

    #[test]
    fn an_enrollment_adds_slot_31_enrolls_the_tpm2_then_removes_slot_31() {
        let shell = Shell::with_header(PASSPHRASE_ONLY);
        let tpm = Tpm::present();

        let run = enroll_with(&shell, &Efivars { secure_boot: 1 }, &tpm, &Flag::default());

        assert_eq!(run.code, 0);
        assert_eq!(
            shell.runs.borrow()[1..],
            [
                format!("keyctl search @u user {DESCRIPTION}"),
                format!(
                    "cryptsetup luksAddKey --batch-mode --volume-key-keyring %user:{DESCRIPTION} --new-key-slot 31 --pbkdf pbkdf2 --pbkdf-force-iterations 1000 {DEVICE} {UNLOCK_KEY}"
                ),
                format!(
                    "systemd-cryptenroll --unlock-key-file={UNLOCK_KEY} --tpm2-device=auto --tpm2-pcrs={PCRS} {DEVICE}"
                ),
                format!("keyctl invalidate {KEY_ID}"),
                wipe(),
            ]
        );
        assert_eq!(
            run.out,
            format!(
                "Enrolled a TPM2 token on {DEVICE} bound to PCRs {PCRS}, the passphrase stays as a fallback\nRemoved the temporary keyslot 31\n"
            )
        );
        assert_eq!(*tpm.written.borrow(), [PathBuf::from(UNLOCK_KEY)]);
        assert_eq!(*tpm.removed.borrow(), [PathBuf::from(UNLOCK_KEY)]);
    }

    #[test]
    fn a_failed_tpm2_enrollment_passes_its_exit_code_after_removing_slot_31() {
        let shell = Shell::with_header(PASSPHRASE_ONLY)
            .answer("systemd-cryptenroll --unlock-key-file", Answer::exited(3));

        let run = enroll(&shell);

        assert_eq!(run.code, 3);
        assert!(shell.ran(&wipe()));
        assert!(shell.ran("keyctl invalidate"));
        assert_eq!(run.out, "Removed the temporary keyslot 31\n");
    }

    #[test]
    fn a_failed_luks_add_key_has_no_slot_31_to_remove() {
        let shell =
            Shell::with_header(PASSPHRASE_ONLY).answer("cryptsetup luksAddKey", Answer::exited(2));

        let run = enroll(&shell);

        assert_eq!(run.code, 2);
        assert!(!shell.ran(&wipe()));
        assert!(shell.ran("keyctl invalidate"));
    }

    #[test]
    fn a_stop_between_luks_add_key_and_the_enrollment_wipes_slot_31_and_the_key() {
        let flag = Flag::default();
        let shell =
            Shell::with_header(PASSPHRASE_ONLY).raise_during("cryptsetup luksAddKey", &flag);
        let tpm = Tpm::present();

        let run = enroll_with(&shell, &Efivars { secure_boot: 1 }, &tpm, &flag);

        assert_eq!(run.code, 143);
        assert!(!shell.ran("systemd-cryptenroll --unlock-key-file"));
        assert!(shell.ran(&format!("keyctl invalidate {KEY_ID}")));
        assert!(shell.ran(&wipe()));
        assert_eq!(*tpm.removed.borrow(), [PathBuf::from(UNLOCK_KEY)]);
        assert_eq!(run.out, "Removed the temporary keyslot 31\n");
    }

    #[test]
    fn a_cleanup_that_fails_exits_1_and_says_so() {
        let shell = Shell::with_header(PASSPHRASE_ONLY);
        let tpm = Tpm {
            removal_fails: true,
            ..Tpm::present()
        };

        let run = enroll_with(&shell, &Efivars { secure_boot: 1 }, &tpm, &Flag::default());

        assert_eq!(run.code, 1);
        assert!(
            run.out
                .ends_with(&format!("Cleanup failed, check the keyslots of {DEVICE}\n"))
        );
    }
}
