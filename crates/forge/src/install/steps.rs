use std::time::Duration;

use super::{Config, Options, Ports, Session, Stop, disks, secrets};
use crate::system::Storage;
use crate::system::{Commands, Invocation};

const UNMOUNT_TIMEOUT: Duration = Duration::from_secs(600);

pub fn install(config: &Config, options: &Options, ports: &mut Ports) -> u8 {
    let mut session = Session { config, ports };
    match steps(&mut session, options) {
        Ok(()) => 0,
        Err(stop) => {
            if let Stop::Refused(message) = &stop {
                let _ = write!(session.ports.out, "\nforge install: {message}\n");
            }
            stop.exit_code()
        }
    }
}

struct Plan {
    disk: String,
    target: String,
    passphrase: Vec<u8>,
    hashes: Vec<(String, String)>,
}

fn steps(session: &mut Session, options: &Options) -> Result<(), Stop> {
    refuse_conflicts(session.config, options)?;
    introduce(session, options);
    let plan = prepare(session, options)?;
    let report_saved = apply(session, options, &plan)?;
    finish(session, options, &plan.disk, report_saved)
}

fn refuse_conflicts(config: &Config, options: &Options) -> Result<(), Stop> {
    let host = config.host_name.as_str();
    if let Some((user, _)) = options
        .password_files
        .iter()
        .find(|(user, _)| !config.has_user(user))
    {
        return Err(Stop::Refused(format!(
            "{user} is not a user of {host} ({}).",
            config.user_list()
        )));
    }

    if options.beside_windows && !config.dual_boot {
        return Err(Stop::Refused(format!(
            "{host} does not import the dual-boot feature, so Windows would be missing from its boot menu. Add \"dual-boot\" to its features, rebuild the installer, then run again."
        )));
    }
    Ok(())
}

fn introduce(session: &mut Session, options: &Options) {
    let host = session.config.host_name.clone();
    session.say(&format!("Install {host}"));
    if options.beside_windows {
        session.print(&format!("This installer keeps Windows and installs {host}, encrypted, in the free space of its disk.\n"));
    } else {
        session.print(&format!(
            "This installer erases one whole disk and installs {host} on it, encrypted.\n"
        ));
    }
}

fn prepare(session: &mut Session, options: &Options) -> Result<Plan, Stop> {
    let config = session.config;
    let disk = match &options.disk {
        Some(disk) => disk.clone(),
        None if options.yes => return Err(Stop::Refused("--yes needs --disk.".to_owned())),
        None => choose_disk(session)?,
    };
    let target = check_disk(session, &disk)?;

    let passphrase = match &options.passphrase_file {
        Some(file) => secrets::read_file(session, file, "the disk passphrase")?,
        None if options.yes => {
            return Err(Stop::Refused("--yes needs --passphrase-file.".to_owned()));
        }
        None => secrets::ask_twice(session, "Disk passphrase, asked at boot to unlock it")?,
    };

    let mut hashes = Vec::new();
    for user in &config.users {
        let password = match options.password_file(user) {
            Some(file) => secrets::read_file(session, file, &format!("the password of {user}"))?,
            None if options.yes => {
                return Err(Stop::Refused(format!(
                    "--yes needs --password-file {user}=FILE."
                )));
            }
            None => secrets::ask_twice(session, &format!("Login password of {user}"))?,
        };
        hashes.push((user.clone(), secrets::hash(session, &password)?));
    }

    if !options.yes {
        confirm(session, &disk, options.beside_windows)?;
    }
    Ok(Plan {
        disk,
        target,
        passphrase,
        hashes,
    })
}

fn apply(session: &mut Session, options: &Options, plan: &Plan) -> Result<bool, Stop> {
    let config = session.config;
    let Plan {
        disk,
        target,
        passphrase,
        hashes,
    } = plan;
    let release = Release {
        commands: session.ports.commands,
        storage: session.ports.storage,
        config,
    };
    release.target();
    session.checkpoint()?;
    let storage = session.ports.storage;
    storage
        .link(target.as_ref(), &config.target_link)
        .map_err(|error| {
            Stop::Refused(format!(
                "cannot link {}: {error}",
                config.target_link.display()
            ))
        })?;
    storage
        .install_file(&config.passphrase_file, passphrase, 0o600)
        .map_err(|error| {
            Stop::Refused(format!(
                "cannot write {}: {error}",
                config.passphrase_file.display()
            ))
        })?;

    let layout = if options.beside_windows {
        session.say(&format!(
            "Creating the encrypted layout beside Windows on {disk}"
        ));
        Invocation::new(
            &config.beside_windows,
            &[
                "dual-boot",
                "install-beside-windows",
                target,
                &config.system,
            ],
        )
    } else {
        session.say(&format!("Erasing {disk} and creating the encrypted layout"));
        Invocation::new(&config.disko, &["--yes-wipe-all-disks"])
    };
    succeed(session, &layout.attached())?;

    secrets::write(session, hashes)?;

    session.say("Recording the hardware report in /persist/facter.json");
    let report = session
        .run(&Invocation::new("nixos-facter", &["-o", "/mnt/persist/facter.json"]).attached())?;
    if !report.success {
        session.print("No hardware report: nixos-facter failed, the install goes on.\n");
    }

    session.say(&format!("Installing {}", config.host_name));
    succeed(
        session,
        &Invocation::new(
            "nixos-install",
            &[
                "--system",
                &config.system,
                "--no-root-passwd",
                "--no-channel-copy",
            ],
        )
        .attached(),
    )?;
    release.target();
    session.checkpoint()?;
    Ok(report.success)
}

fn finish(
    session: &mut Session,
    options: &Options,
    disk: &str,
    report_saved: bool,
) -> Result<(), Stop> {
    let host = session.config.host_name.clone();
    session.say(&format!("{host} is installed on {disk}."));
    if report_saved {
        session.print("Its hardware report is in /persist/facter.json: copy it next to the host file, git add it and add facter = ./facter.json; to the host.\n");
    }
    if !options.reboot {
        return Ok(());
    }

    if !options.yes {
        let answer = session.line("Remove the installer medium, then reboot now? [Y/n] ")?;
        if answer.starts_with(['n', 'N']) {
            session.print("Reboot when ready with: systemctl reboot\n");
            return Ok(());
        }
    }
    succeed(
        session,
        &Invocation::new("systemctl", &["reboot"]).attached(),
    )
}

fn succeed(session: &Session, invocation: &Invocation) -> Result<(), Stop> {
    let answer = session.run(invocation)?;
    if answer.success {
        return Ok(());
    }
    Err(Stop::failed(&answer))
}

fn choose_disk(session: &mut Session) -> Result<String, Stop> {
    let disks = disks::list(session)?;
    if disks.is_empty() {
        return Err(Stop::Refused(
            "no disk to install on: plug one in and run forge install again.".to_owned(),
        ));
    }

    session.say("Disks:");
    for (index, disk) in disks.iter().enumerate() {
        let description = disks::describe(session, disk)?;
        session.print(&format!("  {}) {disk}  ({description})\n", index + 1));
    }
    session.print("\n");

    let prompt = format!("Disk to install on, 1 to {}: ", disks.len());
    loop {
        let choice = session.line(&prompt)?;
        let picked = Some(choice.as_str())
            .filter(|choice| !choice.is_empty() && choice.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|choice| choice.parse::<usize>().ok())
            .and_then(|number| number.checked_sub(1))
            .and_then(|index| disks.get(index));
        if let Some(disk) = picked {
            return Ok(disk.clone());
        }
        session.print("Type a number from the list.\n");
    }
}

fn check_disk(session: &Session, disk: &str) -> Result<String, Stop> {
    if !session.ports.storage.is_block_device(disk.as_ref()) {
        return Err(Stop::Refused(format!("{disk} is not a block device.")));
    }
    let target = session
        .ports
        .files
        .canonicalize(disk.as_ref())
        .display()
        .to_string();
    if !disks::list(session)?.contains(&target) {
        return Err(Stop::Refused(format!(
            "{disk} is not a whole writable disk, or it holds this installer."
        )));
    }

    let others = disks::others_with_layout(session, &target)?;
    if !others.is_empty() {
        let named = others
            .iter()
            .fold(String::new(), |named, other| named + other + " ");
        return Err(Stop::Refused(format!(
            "{named}already carries this layout's partition labels, so the install could format it instead of {disk}. Unplug that disk or erase its partitions (wipefs -a), then run forge install again."
        )));
    }
    Ok(target)
}

fn confirm(session: &mut Session, disk: &str, beside_windows: bool) -> Result<(), Stop> {
    let fate = if beside_windows {
        "Windows kept, NixOS in its free space"
    } else {
        "erased entirely"
    };
    let description = disks::describe(session, disk)?;
    let config = session.config;
    session.say("Ready to install");
    session.print(&format!(
        "  Host:   {}\n  Disk:   {disk} ({description}), {fate}\n  Users:  {}\n\n",
        config.host_name,
        config.user_list()
    ));
    let typed = session.line(&format!("Type {disk} to install on it: "))?;
    if typed != disk {
        return Err(Stop::Refused(
            "the name did not match, nothing was changed.".to_owned(),
        ));
    }
    Ok(())
}

struct Release<'a> {
    commands: &'a dyn Commands,
    storage: &'a dyn Storage,
    config: &'a Config,
}

impl Release<'_> {
    fn target(&self) {
        self.commands.run(
            &Invocation::new("umount", &["-R", "/mnt"])
                .timeout(UNMOUNT_TIMEOUT)
                .discard_output(),
        );
        self.commands.run(
            &Invocation::new("cryptsetup", &["close", &self.config.luks_name]).discard_output(),
        );
    }
}

impl Drop for Release<'_> {
    fn drop(&mut self) {
        self.storage.remove(&self.config.passphrase_file);
        self.storage.remove(&self.config.target_link);
        self.target();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::io::{self, Write as _};
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::install::Parsed;
    use crate::system::interrupt::Flag;
    use crate::system::prompt::{Console, Terminal, Typed};
    use crate::system::{Answer, Files, Owner};

    const HASH: &str = "$y$j9T$salt$hash";

    fn config() -> Config {
        Config {
            host_name: "example".to_owned(),
            system: "/nix/store/system".to_owned(),
            disko: "/nix/store/disko".to_owned(),
            beside_windows: "/nix/store/install-beside-windows".to_owned(),
            dual_boot: false,
            target_link: PathBuf::from("/run/install-host/disk"),
            passphrase_file: PathBuf::from("/tmp/secret.key"),
            luks_name: "crypted".to_owned(),
            users: vec!["alice".to_owned(), "bob".to_owned()],
        }
    }

    const LSBLK: &str = r#"{"blockdevices": [
        {"name": "/dev/sr0", "type": "rom", "size": 1000000, "ro": false},
        {"name": "/dev/vda", "type": "disk", "size": 17179869184, "ro": false},
        {"name": "/dev/vdb", "type": "disk", "size": 8589934592, "ro": false}
    ]}"#;

    const NO_LAYOUT: &str = r#"{"blockdevices": [
        {"name": "/dev/vda", "type": "disk", "pkname": null, "partlabel": null},
        {"name": "/dev/vdb", "type": "disk", "pkname": null, "partlabel": null}
    ]}"#;

    struct Machine {
        answers: HashMap<String, Answer>,
        log: RefCell<Vec<String>>,
        stdin: RefCell<Vec<Vec<u8>>>,
        interrupt_on: Option<(String, Flag, i32)>,
    }

    impl Machine {
        fn new() -> Self {
            let mut machine = Self {
                answers: HashMap::new(),
                log: RefCell::new(Vec::new()),
                stdin: RefCell::new(Vec::new()),
                interrupt_on: None,
            };
            machine.answer("findmnt -no SOURCE /iso", Answer::exited(1));
            machine.answer("lsblk -J -d -p -b -o NAME,TYPE,SIZE,RO", Answer::ok(LSBLK));
            machine.answer(
                "lsblk -J -l -p -o NAME,TYPE,PKNAME,PARTN,PARTTYPE,PARTLABEL",
                Answer::ok(NO_LAYOUT),
            );
            machine.answer("lsblk -dno SIZE /dev/vda", Answer::ok("  16G\n"));
            machine.answer(
                "lsblk -dno MODEL /dev/vda",
                Answer::ok("QEMU HARDDISK   \n"),
            );
            machine.answer("lsblk -dno SIZE /dev/vdb", Answer::ok("8G\n"));
            machine.answer("lsblk -dno MODEL /dev/vdb", Answer::ok("\n"));
            machine.answer(
                "mkpasswd --method=yescrypt --stdin",
                Answer::ok(&format!("{HASH}\n")),
            );
            machine
        }

        fn answer(&mut self, command: &str, answer: Answer) {
            self.answers.insert(command.to_owned(), answer);
        }

        fn ran(&self) -> Vec<String> {
            self.log.borrow().clone()
        }

        fn ran_command(&self, command: &str) -> bool {
            self.log.borrow().iter().any(|line| line == command)
        }

        fn erased(&self) -> bool {
            self.log.borrow().iter().any(|line| {
                line.starts_with("/nix/store/disko")
                    || line.starts_with("/nix/store/install-beside-windows")
                    || line.starts_with("nixos-install")
            })
        }
    }

    impl Commands for Machine {
        fn run(&self, invocation: &Invocation) -> Answer {
            let line = invocation.command_line();
            self.log.borrow_mut().push(line.clone());
            if let Some(input) = &invocation.input {
                self.stdin.borrow_mut().push(input.clone());
            }
            if let Some((command, flag, signal)) = &self.interrupt_on
                && *command == line
            {
                flag.raise(*signal);
                return Answer::exited(128 + signal);
            }
            self.answers
                .get(&line)
                .cloned()
                .unwrap_or_else(|| Answer::ok(""))
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Change {
        Link(PathBuf, PathBuf),
        Dir(PathBuf, u32),
        File(PathBuf, Vec<u8>, u32),
        Removed(PathBuf),
    }

    #[derive(Default)]
    struct Disk {
        block_devices: HashSet<PathBuf>,
        links: HashMap<PathBuf, PathBuf>,
        contents: HashMap<PathBuf, Vec<u8>>,
        changes: RefCell<Vec<Change>>,
    }

    impl Disk {
        fn new() -> Self {
            let mut disk = Self::default();
            for device in ["/dev/sr0", "/dev/vda", "/dev/vdb"] {
                disk.block_devices.insert(PathBuf::from(device));
            }
            disk
        }

        fn file(mut self, path: &str, content: &str) -> Self {
            self.contents
                .insert(PathBuf::from(path), content.as_bytes().to_vec());
            self
        }

        fn changes(&self) -> Vec<Change> {
            self.changes.borrow().clone()
        }

        fn record(&self, change: Change) {
            self.changes.borrow_mut().push(change);
        }
    }

    impl Files for Disk {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            self.contents
                .get(path)
                .cloned()
                .ok_or_else(|| io::ErrorKind::NotFound.into())
        }

        fn read_link(&self, path: &Path) -> Option<PathBuf> {
            self.links.get(path).cloned()
        }

        fn is_dir(&self, _: &Path) -> bool {
            false
        }

        fn list(&self, _: &Path) -> Vec<String> {
            Vec::new()
        }

        fn canonicalize(&self, path: &Path) -> PathBuf {
            self.links
                .get(path)
                .cloned()
                .unwrap_or_else(|| path.to_path_buf())
        }

        fn owner(&self, _: &Path) -> Option<Owner> {
            None
        }
    }

    impl Storage for Disk {
        fn is_block_device(&self, path: &Path) -> bool {
            self.block_devices.contains(&self.canonicalize(path))
        }

        fn exists(&self, path: &Path) -> bool {
            self.contents.contains_key(path)
        }

        fn link(&self, target: &Path, link: &Path) -> io::Result<()> {
            self.record(Change::Link(target.to_path_buf(), link.to_path_buf()));
            Ok(())
        }

        fn create_new_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
            self.install_dir(path, mode)
        }

        fn install_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
            self.record(Change::Dir(path.to_path_buf(), mode));
            Ok(())
        }

        fn install_file(&self, path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
            self.record(Change::File(path.to_path_buf(), bytes.to_vec(), mode));
            Ok(())
        }

        fn create_dirs(&self, path: &Path) -> io::Result<()> {
            self.record(Change::Dir(path.to_path_buf(), 0o755));
            Ok(())
        }

        fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
            self.record(Change::File(path.to_path_buf(), bytes.to_vec(), 0o644));
            Ok(())
        }

        fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
            Ok(())
        }

        fn remove(&self, path: &Path) {
            self.record(Change::Removed(path.to_path_buf()));
        }

        fn remove_dir(&self, path: &Path) -> io::Result<()> {
            self.record(Change::Removed(path.to_path_buf()));
            Ok(())
        }
    }

    struct Outcome {
        code: u8,
        printed: String,
    }

    fn options(args: &[&str]) -> Options {
        let args: Vec<_> = args.iter().map(std::ffi::OsString::from).collect();
        match crate::install::parse(&args) {
            Ok(Parsed::Run(options)) => options,
            other => panic!("unexpected {other:?}"),
        }
    }

    fn run_with(
        config: &Config,
        args: &[&str],
        machine: &Machine,
        disk: &Disk,
        terminal: &mut dyn Terminal,
        flag: &Flag,
    ) -> Outcome {
        let mut out = Vec::new();
        let mut ports = Ports {
            commands: machine,
            files: disk,
            storage: disk,
            terminal,
            interrupt: flag,
            out: &mut out,
        };
        let code = install(config, &options(args), &mut ports);
        Outcome {
            code,
            printed: String::from_utf8(out).unwrap(),
        }
    }

    fn run(args: &[&str], machine: &Machine, disk: &Disk, terminal: &mut Typed) -> Outcome {
        run_with(&config(), args, machine, disk, terminal, &Flag::default())
    }

    fn with_files() -> Disk {
        Disk::new()
            .file("/tmp/pass", "disk-secret\n\n")
            .file("/tmp/alice", "alice-password\n")
            .file("/tmp/bob", "bob-password")
    }

    const UNATTENDED: [&str; 10] = [
        "--disk",
        "/dev/vda",
        "--passphrase-file",
        "/tmp/pass",
        "--password-file",
        "alice=/tmp/alice",
        "--password-file",
        "bob=/tmp/bob",
        "--yes",
        "--no-reboot",
    ];

    fn cleanup() -> Vec<Change> {
        vec![
            Change::Removed(PathBuf::from("/tmp/secret.key")),
            Change::Removed(PathBuf::from("/run/install-host/disk")),
        ]
    }

    #[test]
    fn an_unattended_install_runs_every_step_in_order_and_cleans_up() {
        let machine = Machine::new();
        let disk = with_files();

        let outcome = run(&UNATTENDED, &machine, &disk, &mut Typed::new(&[]));

        assert_eq!(outcome.code, 0, "{}", outcome.printed);
        let ran: Vec<String> = machine
            .ran()
            .into_iter()
            .filter(|line| !line.starts_with("lsblk") && !line.starts_with("findmnt"))
            .collect();
        assert_eq!(
            ran,
            [
                "mkpasswd --method=yescrypt --stdin",
                "mkpasswd --method=yescrypt --stdin",
                "umount -R /mnt",
                "cryptsetup close crypted",
                "/nix/store/disko --yes-wipe-all-disks",
                "nixos-facter -o /mnt/persist/facter.json",
                "nixos-install --system /nix/store/system --no-root-passwd --no-channel-copy",
                "umount -R /mnt",
                "cryptsetup close crypted",
                "umount -R /mnt",
                "cryptsetup close crypted",
            ]
        );
        assert_eq!(
            *machine.stdin.borrow(),
            [b"alice-password".to_vec(), b"bob-password".to_vec()]
        );
        let mut expected = vec![
            Change::Link(
                PathBuf::from("/dev/vda"),
                PathBuf::from("/run/install-host/disk"),
            ),
            Change::File(
                PathBuf::from("/tmp/secret.key"),
                b"disk-secret".to_vec(),
                0o600,
            ),
            Change::Dir(PathBuf::from("/mnt/persist/passwords"), 0o700),
            Change::File(
                PathBuf::from("/mnt/persist/passwords/alice"),
                format!("{HASH}\n").into_bytes(),
                0o600,
            ),
            Change::File(
                PathBuf::from("/mnt/persist/passwords/bob"),
                format!("{HASH}\n").into_bytes(),
                0o600,
            ),
        ];
        expected.extend(cleanup());
        assert_eq!(disk.changes(), expected);
        assert!(outcome.printed.contains("\nInstall example\nThis installer erases one whole disk and installs example on it, encrypted.\n"));
        assert!(outcome.printed.contains(
            "\nexample is installed on /dev/vda.\nIts hardware report is in /persist/facter.json"
        ));
    }

    #[test]
    fn a_password_file_for_an_unknown_user_is_refused_before_anything_runs() {
        let machine = Machine::new();

        let outcome = run(
            &["--password-file", "carol=/tmp/carol"],
            &machine,
            &with_files(),
            &mut Typed::new(&[]),
        );

        assert_eq!(outcome.code, 1);
        assert_eq!(
            outcome.printed,
            "\nforge install: carol is not a user of example (alice bob).\n"
        );
        assert!(machine.ran().is_empty());
    }

    #[test]
    fn beside_windows_is_refused_on_a_host_without_dual_boot() {
        let outcome = run(
            &["--beside-windows"],
            &Machine::new(),
            &with_files(),
            &mut Typed::new(&[]),
        );

        assert_eq!(outcome.code, 1);
        assert!(outcome.printed.contains("forge install: example does not import the dual-boot feature, so Windows would be missing from its boot menu. Add \"dual-boot\" to its features, rebuild the installer, then run again.\n"));
    }

    #[test]
    fn beside_windows_hands_the_resolved_disk_and_the_system_to_its_script() {
        let machine = Machine::new();
        let mut args = UNATTENDED.to_vec();
        args.push("--beside-windows");
        let config = Config {
            dual_boot: true,
            ..config()
        };

        let outcome = run_with(
            &config,
            &args,
            &machine,
            &with_files(),
            &mut Typed::new(&[]),
            &Flag::default(),
        );

        assert_eq!(outcome.code, 0, "{}", outcome.printed);
        assert!(
            machine.ran_command(
                "/nix/store/install-beside-windows dual-boot install-beside-windows /dev/vda /nix/store/system"
            )
        );
        assert!(!machine.ran_command("/nix/store/disko --yes-wipe-all-disks"));
        assert!(
            outcome
                .printed
                .contains("\nCreating the encrypted layout beside Windows on /dev/vda\n")
        );
    }

    #[test]
    fn yes_without_a_disk_is_refused() {
        let outcome = run(
            &["--yes"],
            &Machine::new(),
            &with_files(),
            &mut Typed::new(&[]),
        );

        assert_eq!(outcome.code, 1);
        assert!(
            outcome
                .printed
                .ends_with("\nforge install: --yes needs --disk.\n")
        );
    }

    #[test]
    fn a_path_that_is_not_a_block_device_is_refused() {
        let outcome = run(
            &["--disk", "/dev/nope", "--yes"],
            &Machine::new(),
            &with_files(),
            &mut Typed::new(&[]),
        );

        assert_eq!(outcome.code, 1);
        assert!(
            outcome
                .printed
                .ends_with("forge install: /dev/nope is not a block device.\n")
        );
    }

    #[test]
    fn a_block_device_that_is_not_a_candidate_is_refused() {
        let machine = Machine::new();

        let outcome = run(
            &["--disk", "/dev/sr0", "--yes"],
            &machine,
            &with_files(),
            &mut Typed::new(&[]),
        );

        assert_eq!(outcome.code, 1);
        assert!(outcome.printed.ends_with(
            "forge install: /dev/sr0 is not a whole writable disk, or it holds this installer.\n"
        ));
        assert!(!machine.erased());
    }

    #[test]
    fn the_disk_holding_the_live_media_is_not_a_candidate() {
        let mut machine = Machine::new();
        machine.answer("findmnt -no SOURCE /iso", Answer::ok("/dev/loop0\n"));
        machine.answer("losetup -nO BACK-FILE /dev/loop0", Answer::ok("/iso.img\n"));
        machine.answer("findmnt -no SOURCE -T /iso.img", Answer::ok("/dev/vdb1\n"));
        machine.answer(
            "lsblk -nprs -o NAME /dev/vdb1",
            Answer::ok("/dev/vdb1\n/dev/vdb\n"),
        );
        let mut disk = with_files();
        disk.block_devices.insert(PathBuf::from("/dev/vdb1"));

        let outcome = run(
            &["--disk", "/dev/vdb", "--yes"],
            &machine,
            &disk,
            &mut Typed::new(&[]),
        );

        assert!(outcome.printed.ends_with(
            "forge install: /dev/vdb is not a whole writable disk, or it holds this installer.\n"
        ));
    }

    #[test]
    fn another_disk_carrying_the_layout_labels_is_refused_by_name() {
        let mut machine = Machine::new();
        machine.answer(
            "lsblk -J -l -p -o NAME,TYPE,PKNAME,PARTN,PARTTYPE,PARTLABEL",
            Answer::ok(
                r#"{"blockdevices": [
                {"name": "/dev/vdb1", "type": "part", "pkname": "/dev/vdb", "partlabel": "disk-main-ESP"},
                {"name": "/dev/vdb2", "type": "part", "pkname": "/dev/vdb", "partlabel": "disk-main-luks"}
            ]}"#,
            ),
        );

        let outcome = run(&UNATTENDED, &machine, &with_files(), &mut Typed::new(&[]));

        assert_eq!(outcome.code, 1);
        assert!(outcome.printed.ends_with("\nforge install: /dev/vdb1 /dev/vdb2 already carries this layout's partition labels, so the install could format it instead of /dev/vda. Unplug that disk or erase its partitions (wipefs -a), then run forge install again.\n"));
        assert!(!machine.erased());
    }

    #[test]
    fn a_symlinked_disk_with_spaces_resolves_like_realpath_and_is_installed() {
        let machine = Machine::new();
        let mut disk = with_files();
        let by_id = "/dev/disk/by-id/virtio-My Disk";
        disk.links
            .insert(PathBuf::from(by_id), PathBuf::from("/dev/vda"));
        let mut args = UNATTENDED.to_vec();
        args[1] = by_id;
        args.retain(|&arg| arg != "--yes");

        let mut terminal = Typed::new(&[by_id]);
        let outcome = run(&args, &machine, &disk, &mut terminal);

        assert_eq!(outcome.code, 0, "{}", outcome.printed);
        assert_eq!(
            terminal.prompts,
            [format!("Type {by_id} to install on it: ")]
        );
        assert!(outcome.printed.contains(&format!(
            "  Disk:   {by_id} (, unknown model), erased entirely\n"
        )));
        assert_eq!(
            disk.changes()[0],
            Change::Link(
                PathBuf::from("/dev/vda"),
                PathBuf::from("/run/install-host/disk")
            )
        );
    }

    #[test]
    fn the_disk_list_numbers_each_candidate_and_asks_again_until_a_valid_number() {
        let machine = Machine::new();
        let mut terminal =
            Typed::new(&["0", "two", "3", "2"]).then_fail(io::ErrorKind::UnexpectedEof);

        let outcome = run(&[], &machine, &with_files(), &mut terminal);

        assert!(outcome.printed.contains(
            "\nDisks:\n  1) /dev/vda  (16G, QEMU HARDDISK)\n  2) /dev/vdb  (8G, unknown model)\n\n"
        ));
        assert_eq!(
            outcome
                .printed
                .matches("Type a number from the list.\n")
                .count(),
            3
        );
        assert_eq!(terminal.prompts[..4], ["Disk to install on, 1 to 2: "; 4]);
        assert_eq!(
            terminal.prompts[4],
            "Disk passphrase, asked at boot to unlock it"
        );
        assert_eq!(outcome.code, 1);
    }

    #[test]
    fn no_candidate_disk_stops_with_a_hint() {
        let mut machine = Machine::new();
        machine.answer(
            "lsblk -J -d -p -b -o NAME,TYPE,SIZE,RO",
            Answer::ok(r#"{"blockdevices": []}"#),
        );

        let outcome = run(&[], &machine, &with_files(), &mut Typed::new(&[]));

        assert_eq!(outcome.code, 1);
        assert!(outcome.printed.ends_with(
            "forge install: no disk to install on: plug one in and run forge install again.\n"
        ));
    }

    #[test]
    fn a_secret_is_asked_twice_until_both_match_and_are_not_empty() {
        let machine = Machine::new();
        let disk = with_files();
        let mut terminal = Typed::new(&[
            "",
            "",
            "one",
            "two",
            "disk-secret",
            "disk-secret",
            "a",
            "a",
            "b",
            "b",
            "no",
        ]);

        let outcome = run(&["--disk", "/dev/vda"], &machine, &disk, &mut terminal);

        assert!(
            outcome
                .printed
                .contains("It cannot be empty.\nThey differ, try again.\n")
        );
        assert_eq!(
            terminal.prompts[..8],
            [
                "Disk passphrase, asked at boot to unlock it",
                "Same again",
                "Disk passphrase, asked at boot to unlock it",
                "Same again",
                "Disk passphrase, asked at boot to unlock it",
                "Same again",
                "Login password of alice",
                "Same again",
            ]
        );
        assert_eq!(*machine.stdin.borrow(), [b"a".to_vec(), b"b".to_vec()]);
        assert_eq!(outcome.code, 1);
        assert!(
            outcome
                .printed
                .ends_with("forge install: the name did not match, nothing was changed.\n")
        );
    }

    #[test]
    fn a_confirmation_that_does_not_match_changes_nothing() {
        let machine = Machine::new();
        let disk = with_files();
        let mut args = UNATTENDED.to_vec();
        args.retain(|&arg| arg != "--yes");

        let outcome = run(&args, &machine, &disk, &mut Typed::new(&["/dev/vdb"]));

        assert_eq!(outcome.code, 1);
        assert!(outcome.printed.contains("\nReady to install\n  Host:   example\n  Disk:   /dev/vda (16G, QEMU HARDDISK), erased entirely\n  Users:  alice bob\n\n"));
        assert!(disk.changes().is_empty());
        assert!(!machine.ran_command("umount -R /mnt"));
        assert!(!machine.erased());
    }

    #[test]
    fn yes_without_a_passphrase_or_a_password_file_stops_before_erasing() {
        let machine = Machine::new();
        let disk = with_files();

        let no_passphrase = run(
            &["--disk", "/dev/vda", "--yes"],
            &machine,
            &disk,
            &mut Typed::new(&[]),
        );
        let no_password = run(
            &[
                "--disk",
                "/dev/vda",
                "--passphrase-file",
                "/tmp/pass",
                "--password-file",
                "alice=/tmp/alice",
                "--yes",
            ],
            &machine,
            &disk,
            &mut Typed::new(&[]),
        );

        assert!(
            no_passphrase
                .printed
                .ends_with("forge install: --yes needs --passphrase-file.\n")
        );
        assert!(
            no_password
                .printed
                .ends_with("forge install: --yes needs --password-file bob=FILE.\n")
        );
        assert!(disk.changes().is_empty());
        assert!(!machine.erased());
    }

    #[test]
    fn a_secret_file_must_be_readable_and_not_empty() {
        let disk = with_files().file("/tmp/empty", "\n");
        let unreadable = run(
            &[
                "--disk",
                "/dev/vda",
                "--passphrase-file",
                "/tmp/missing",
                "--yes",
            ],
            &Machine::new(),
            &disk,
            &mut Typed::new(&[]),
        );
        let empty = run(
            &[
                "--disk",
                "/dev/vda",
                "--passphrase-file",
                "/tmp/pass",
                "--password-file",
                "alice=/tmp/empty",
                "--yes",
            ],
            &Machine::new(),
            &disk,
            &mut Typed::new(&[]),
        );

        assert!(
            unreadable
                .printed
                .ends_with("forge install: cannot read /tmp/missing.\n")
        );
        assert!(empty.printed.ends_with(
            "forge install: /tmp/empty is empty: it must hold the password of alice.\n"
        ));
    }

    #[test]
    fn secrets_piped_twice_each_are_read_like_the_shell_read_s() {
        let machine = Machine::new();
        let disk = with_files();
        let (reader, mut writer) = io::pipe().unwrap();
        writer
            .write_all(b"disk-secret\ndisk-secret\nalice-pw\nalice-pw\nbob-pw\nbob-pw\n/dev/vda\n")
            .unwrap();
        let mut console = Console::new(reader, Vec::new(), None);

        let outcome = run_with(
            &config(),
            &["--disk", "/dev/vda", "--no-reboot"],
            &machine,
            &disk,
            &mut console,
            &Flag::default(),
        );

        assert_eq!(outcome.code, 0, "{}", outcome.printed);
        assert_eq!(
            *machine.stdin.borrow(),
            [b"alice-pw".to_vec(), b"bob-pw".to_vec()]
        );
        assert!(disk.changes().contains(&Change::File(
            PathBuf::from("/tmp/secret.key"),
            b"disk-secret".to_vec(),
            0o600
        )));
        assert!(machine.ran_command(
            "nixos-install --system /nix/store/system --no-root-passwd --no-channel-copy"
        ));
    }

    #[test]
    fn an_interrupted_prompt_exits_130_without_touching_the_disk() {
        let machine = Machine::new();
        let disk = with_files();
        let mut terminal = Typed::new(&[]).then_fail(io::ErrorKind::Interrupted);

        let outcome = run(&["--disk", "/dev/vda"], &machine, &disk, &mut terminal);

        assert_eq!(outcome.code, 130);
        assert!(!outcome.printed.contains("forge install:"));
        assert!(disk.changes().is_empty());
    }

    #[test]
    fn an_interrupted_nixos_install_releases_the_target_and_removes_the_passphrase_then_exits_130()
    {
        let flag = Flag::default();
        let mut machine = Machine::new();
        let install = "nixos-install --system /nix/store/system --no-root-passwd --no-channel-copy";
        machine.interrupt_on = Some((install.to_owned(), flag.clone(), 2));
        let disk = with_files();

        let outcome = run_with(
            &config(),
            &UNATTENDED,
            &machine,
            &disk,
            &mut Typed::new(&[]),
            &flag,
        );

        assert_eq!(outcome.code, 130);
        let ran = machine.ran();
        let after: Vec<&str> = ran
            .iter()
            .skip_while(|line| *line != install)
            .skip(1)
            .map(String::as_str)
            .collect();
        assert_eq!(after, ["umount -R /mnt", "cryptsetup close crypted"]);
        assert!(disk.changes().ends_with(&cleanup()));
    }

    #[test]
    fn a_stop_request_during_the_layout_exits_143_and_cleans_up() {
        let flag = Flag::default();
        let mut machine = Machine::new();
        machine.interrupt_on = Some((
            "/nix/store/disko --yes-wipe-all-disks".to_owned(),
            flag.clone(),
            15,
        ));
        let disk = with_files();

        let outcome = run_with(
            &config(),
            &UNATTENDED,
            &machine,
            &disk,
            &mut Typed::new(&[]),
            &flag,
        );

        assert_eq!(outcome.code, 143);
        assert!(!machine.ran_command(
            "nixos-install --system /nix/store/system --no-root-passwd --no-channel-copy"
        ));
        assert!(disk.changes().ends_with(&cleanup()));
    }

    #[test]
    fn a_failed_layout_exits_with_its_code_and_cleans_up() {
        let mut machine = Machine::new();
        machine.answer("/nix/store/disko --yes-wipe-all-disks", Answer::exited(3));
        let disk = with_files();

        let outcome = run(&UNATTENDED, &machine, &disk, &mut Typed::new(&[]));

        assert_eq!(outcome.code, 3);
        assert!(!machine.ran_command(
            "nixos-install --system /nix/store/system --no-root-passwd --no-channel-copy"
        ));
        assert!(disk.changes().ends_with(&cleanup()));
    }

    #[test]
    fn a_failed_hardware_report_only_warns() {
        let mut machine = Machine::new();
        machine.answer(
            "nixos-facter -o /mnt/persist/facter.json",
            Answer::exited(1),
        );

        let outcome = run(&UNATTENDED, &machine, &with_files(), &mut Typed::new(&[]));

        assert_eq!(outcome.code, 0);
        assert!(
            outcome
                .printed
                .contains("No hardware report: nixos-facter failed, the install goes on.\n")
        );
        assert!(!outcome.printed.contains("Its hardware report"));
        assert!(machine.ran_command(
            "nixos-install --system /nix/store/system --no-root-passwd --no-channel-copy"
        ));
    }

    #[test]
    fn the_reboot_question_answered_no_stays_on_the_installer() {
        let machine = Machine::new();
        let mut args = UNATTENDED.to_vec();
        args.retain(|&arg| arg != "--yes" && arg != "--no-reboot");
        let mut terminal = Typed::new(&["/dev/vda", "No"]);

        let outcome = run(&args, &machine, &with_files(), &mut terminal);

        assert_eq!(outcome.code, 0);
        assert_eq!(
            terminal.prompts[1],
            "Remove the installer medium, then reboot now? [Y/n] "
        );
        assert!(
            outcome
                .printed
                .ends_with("Reboot when ready with: systemctl reboot\n")
        );
        assert!(!machine.ran_command("systemctl reboot"));
    }

    #[test]
    fn the_reboot_comes_by_default_and_with_yes_without_a_question() {
        let machine = Machine::new();
        let mut args = UNATTENDED.to_vec();
        args.retain(|&arg| arg != "--no-reboot");
        let mut terminal = Typed::new(&[]);

        let outcome = run(&args, &machine, &with_files(), &mut terminal);

        assert_eq!(outcome.code, 0);
        assert!(terminal.prompts.is_empty());
        assert!(machine.ran_command("systemctl reboot"));
    }
}
