use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::config::SyncSettings;
use crate::system::mount::Mounts;
use crate::system::partitions::{self, Partition};
use crate::system::{Commands, Files, Invocation, Storage, efi, firmware};

const BOOT_MANAGER: &str = "EFI/Microsoft/Boot/bootmgfw.efi";
const COPY_MARK: &str = "EFI/Microsoft/Boot/copied-by-nixos";
const LINUX_LABEL: &str = "Linux Boot Manager";
const LINUX_LOADER: &str = "\\EFI\\systemd\\systemd-bootx64.efi";
const WINDOWS_LABEL: &str = "Windows Boot Manager";
const WINDOWS_LOADER: &str = "\\EFI\\Microsoft\\Boot\\bootmgfw.efi";
const MOUNT_OPTIONS: &str = "ro,nodev,nosuid,noexec";

pub struct Ports<'a> {
    pub commands: &'a dyn Commands,
    pub files: &'a dyn Files,
    pub storage: &'a dyn Storage,
    pub mounts: &'a dyn Mounts,
    pub out: &'a mut dyn Write,
}

impl Ports<'_> {
    fn say(&mut self, text: &str) {
        let _ = writeln!(self.out, "forge dual-boot: {text}");
    }
}

struct Esp<'s> {
    mount: &'s Path,
    copy: PathBuf,
    mark: PathBuf,
}

pub fn sync(settings: &SyncSettings, ports: &mut Ports) {
    let mount = settings.esp_mount.as_path();
    let Some(our_esp) = mounted_esp(ports, mount) else {
        ports.say(&format!(
            "no ESP is mounted on {}, nothing to do.",
            mount.display()
        ));
        return;
    };
    let partitions = match partitions::list(ports.commands, None) {
        Ok(partitions) => partitions,
        Err(error) => {
            ports.say(&format!("cannot list the partitions: {error}"));
            return;
        }
    };
    let esp = Esp {
        mount,
        copy: mount.join(BOOT_MANAGER),
        mark: mount.join(COPY_MARK),
    };

    let windows_esp = if ports.storage.exists(&esp.copy) && !ports.storage.exists(&esp.mark) {
        Some(our_esp.clone())
    } else {
        scan(ports, &esp, &partitions, &our_esp)
    };
    let Some(windows_esp) = windows_esp else {
        return;
    };
    if !efi::is_uefi(ports.files) {
        return;
    }
    if order_entries(ports, &partitions, &our_esp, &windows_esp).is_none() {
        ports.say("could not set the firmware boot entries: pick Windows in the menu once its firmware entry exists.");
    }
}

fn mounted_esp(ports: &Ports, mount: &Path) -> Option<String> {
    let mount = mount.display().to_string();
    let found = ports.commands.run(&Invocation::new(
        "findmnt",
        &["-nvo", "SOURCE", "--mountpoint", &mount],
    ));
    let source = found.line().trim();
    if !found.success || source.is_empty() {
        return None;
    }
    Some(
        ports
            .files
            .canonicalize(Path::new(source))
            .display()
            .to_string(),
    )
}

enum Probe {
    Holds,
    Lacks,
    Unreadable,
}

fn scan(ports: &mut Ports, esp: &Esp, partitions: &[Partition], our_esp: &str) -> Option<String> {
    let mut all_read = true;
    for partition in partitions
        .iter()
        .filter(|partition| partition.is_esp() && partition.name != our_esp)
    {
        match copy_boot_manager(ports, esp, &partition.name) {
            Probe::Holds => return Some(partition.name.clone()),
            Probe::Lacks => {}
            Probe::Unreadable => all_read = false,
        }
    }
    if all_read && ports.storage.exists(&esp.mark) {
        remove_copy(ports, esp);
        ports.say("no other disk holds Windows any more: removed the copy of its boot manager.");
    }
    None
}

fn copy_boot_manager(ports: &mut Ports, esp: &Esp, device: &str) -> Probe {
    let Ok(mounted) = ports.mounts.mount_read_only(device, MOUNT_OPTIONS) else {
        return Probe::Unreadable;
    };
    let source = mounted.path().join(BOOT_MANAGER);
    if !ports.storage.exists(&source) {
        return Probe::Lacks;
    }
    let message = match copy_if_changed(ports, &source, esp) {
        Ok(false) => return Probe::Holds,
        Ok(true) => format!(
            "copied the Windows boot manager of {device} to {}, so the menu shows Windows.",
            esp.copy.display()
        ),
        Err(_) => format!(
            "could not copy the Windows boot manager of {device} to {}.",
            esp.mount.display()
        ),
    };
    drop(mounted);
    ports.say(&message);
    Probe::Holds
}

fn copy_if_changed(ports: &Ports, source: &Path, esp: &Esp) -> io::Result<bool> {
    let bytes = ports.files.read(source)?;
    if ports.files.read(&esp.copy).is_ok_and(|copy| copy == bytes) {
        return Ok(false);
    }
    let staged = esp.copy.with_extension("efi.new");
    if let Some(parent) = esp.copy.parent() {
        ports.storage.create_dirs(parent)?;
    }
    ports.storage.write(&staged, &bytes)?;
    ports.storage.rename(&staged, &esp.copy)?;
    ports.storage.write(&esp.mark, &[])?;
    Ok(true)
}

fn remove_copy(ports: &Ports, esp: &Esp) {
    ports.storage.remove(&esp.copy);
    ports.storage.remove(&esp.mark);
    let mut dir = esp.copy.parent();
    while let Some(current) = dir.filter(|dir| *dir != esp.mount && dir.starts_with(esp.mount)) {
        if ports.storage.remove_dir(current).is_err() {
            return;
        }
        dir = current.parent();
    }
}

fn ensure_entry(
    ports: &mut Ports,
    partitions: &[Partition],
    label: &str,
    loader: &str,
    esp: &str,
) -> Option<String> {
    if let Some(number) = firmware::entry_number(ports.commands, label) {
        return Some(number);
    }
    let partition = partitions.iter().find(|partition| partition.name == esp)?;
    let disk = partition.disk.as_deref()?;
    let part = partition.number?.to_string();
    let created = ports.commands.run(&Invocation::new(
        "efibootmgr",
        &[
            "--create-only",
            "--disk",
            disk,
            "--part",
            &part,
            "--label",
            label,
            "--loader",
            loader,
        ],
    ));
    if !created.success {
        return None;
    }
    ports.say(&format!("created the firmware entry {label}."));
    firmware::entry_number(ports.commands, label)
}

fn order_entries(
    ports: &mut Ports,
    partitions: &[Partition],
    our_esp: &str,
    windows_esp: &str,
) -> Option<()> {
    let linux = ensure_entry(ports, partitions, LINUX_LABEL, LINUX_LOADER, our_esp)?;
    let windows = ensure_entry(
        ports,
        partitions,
        WINDOWS_LABEL,
        WINDOWS_LOADER,
        windows_esp,
    )?;
    let current = firmware::boot_order(ports.commands);
    let wanted = [linux.as_str(), windows.as_str()]
        .into_iter()
        .chain(
            current
                .iter()
                .map(String::as_str)
                .filter(|entry| *entry != linux && *entry != windows),
        )
        .collect::<Vec<_>>();
    if wanted == current {
        return Some(());
    }
    let wanted = wanted.join(",");
    let ordered = ports
        .commands
        .run(&Invocation::new("efibootmgr", &["--bootorder", &wanted]));
    if !ordered.success {
        return None;
    }
    ports.say(&format!(
        "firmware boot order set to {wanted}, NixOS first and Windows next."
    ));
    Some(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::io;
    use std::path::PathBuf;

    use super::*;
    use crate::system::mount::ReadOnlyMount;
    use crate::system::{Answer, Invocation, Owner};

    const FINDMNT: &str = "findmnt -nvo SOURCE --mountpoint /boot";
    const LSBLK: &str = "lsblk -J -l -p -o NAME,TYPE,PKNAME,PARTN,PARTTYPE,PARTLABEL";
    const ESP: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
    const COPY: &str = "/boot/EFI/Microsoft/Boot/bootmgfw.efi";
    const MARK: &str = "/boot/EFI/Microsoft/Boot/copied-by-nixos";
    const EFI: &str = "/sys/firmware/efi";
    const CREATE_WINDOWS: &str = "efibootmgr --create-only --disk /dev/vdb --part 1 --label Windows Boot Manager --loader \\EFI\\Microsoft\\Boot\\bootmgfw.efi";

    fn partition(name: &str, disk: &str, number: u32, part_type: &str) -> String {
        format!(
            r#"{{"name":"{name}","type":"part","pkname":"{disk}","partn":{number},"parttype":"{part_type}","partlabel":null}}"#
        )
    }

    fn lsblk(esps: &[&str]) -> String {
        let mut rows = vec![
            r#"{"name":"/dev/vda","type":"disk","pkname":null,"partn":null,"parttype":null,"partlabel":null}"#.to_owned(),
            partition("/dev/vda1", "/dev/vda", 1, ESP),
            partition("/dev/vda2", "/dev/vda", 2, "0fc63daf-8483-4772-8e79-3d69d8477de4"),
        ];
        for esp in esps {
            let disk = esp.trim_end_matches(|c: char| c.is_ascii_digit());
            rows.push(partition(esp, disk, 1, &ESP.to_uppercase()));
        }
        format!(r#"{{"blockdevices": [{}]}}"#, rows.join(","))
    }

    fn efibootmgr(entries: &[(&str, &str)], order: &str) -> String {
        let listed = entries
            .iter()
            .map(|(number, label)| {
                format!("Boot{number}* {label}\tHD(1,GPT,abc)/File(\\EFI\\x.efi)")
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("BootCurrent: 0001\nTimeout: 0 seconds\nBootOrder: {order}\n{listed}\n")
    }

    struct Machine {
        answers: RefCell<HashMap<String, VecDeque<Answer>>>,
        log: RefCell<Vec<String>>,
    }

    impl Machine {
        fn new(esps: &[&str]) -> Self {
            let machine = Self {
                answers: RefCell::new(HashMap::new()),
                log: RefCell::new(Vec::new()),
            };
            machine.answer(
                FINDMNT,
                Answer::ok("/dev/disk/by-partlabel/disk-main-ESP\n"),
            );
            machine.answer(LSBLK, Answer::ok(&lsblk(esps)));
            machine
        }

        fn answer(&self, command: &str, answer: Answer) {
            self.answers
                .borrow_mut()
                .entry(command.to_owned())
                .or_default()
                .push_back(answer);
        }

        fn replace(&self, command: &str, answer: Answer) {
            self.answers
                .borrow_mut()
                .insert(command.to_owned(), VecDeque::from([answer]));
        }

        fn ran(&self) -> Vec<String> {
            self.log.borrow().clone()
        }

        fn ran_starting(&self, prefix: &str) -> Vec<String> {
            self.ran()
                .into_iter()
                .filter(|line| line.starts_with(prefix))
                .collect()
        }
    }

    impl Commands for Machine {
        fn run(&self, invocation: &Invocation) -> Answer {
            let line = invocation.command_line();
            self.log.borrow_mut().push(line.clone());
            let mut answers = self.answers.borrow_mut();
            let Some(queue) = answers.get_mut(&line) else {
                return Answer::ok("");
            };
            if queue.len() > 1 {
                return queue.pop_front().unwrap();
            }
            queue.front().cloned().unwrap()
        }
    }

    #[derive(Default)]
    struct Disk {
        files: RefCell<HashMap<PathBuf, Vec<u8>>>,
        dirs: RefCell<HashSet<PathBuf>>,
        efivars: bool,
        writes_fail: bool,
    }

    impl Disk {
        fn new() -> Self {
            let disk = Self::default();
            disk.dir("/boot/EFI/systemd");
            disk
        }

        fn dir(&self, path: &str) {
            let mut dirs = self.dirs.borrow_mut();
            for ancestor in Path::new(path).ancestors() {
                dirs.insert(ancestor.to_path_buf());
            }
        }

        fn file(&self, path: &str, bytes: &[u8]) {
            let path = PathBuf::from(path);
            if let Some(parent) = path.parent() {
                self.dir(&parent.display().to_string());
            }
            self.files.borrow_mut().insert(path, bytes.to_vec());
        }

        fn content(&self, path: &str) -> Option<Vec<u8>> {
            self.files.borrow().get(Path::new(path)).cloned()
        }

        fn has_dir(&self, path: &str) -> bool {
            self.dirs.borrow().contains(Path::new(path))
        }
    }

    impl Files for Disk {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .ok_or_else(|| io::ErrorKind::NotFound.into())
        }

        fn read_link(&self, _: &Path) -> Option<PathBuf> {
            None
        }

        fn is_dir(&self, path: &Path) -> bool {
            if path == Path::new(EFI) {
                return self.efivars;
            }
            self.dirs.borrow().contains(path)
        }

        fn list(&self, _: &Path) -> Vec<String> {
            Vec::new()
        }

        fn canonicalize(&self, path: &Path) -> PathBuf {
            if path == Path::new("/dev/disk/by-partlabel/disk-main-ESP") {
                return PathBuf::from("/dev/vda1");
            }
            path.to_path_buf()
        }

        fn owner(&self, _: &Path) -> Option<Owner> {
            None
        }
    }

    impl Storage for Disk {
        fn is_block_device(&self, _: &Path) -> bool {
            false
        }

        fn exists(&self, path: &Path) -> bool {
            self.files.borrow().contains_key(path) || self.dirs.borrow().contains(path)
        }

        fn link(&self, _: &Path, _: &Path) -> io::Result<()> {
            Ok(())
        }

        fn create_new_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
            self.install_dir(path, mode)
        }

        fn install_dir(&self, _: &Path, _: u32) -> io::Result<()> {
            Ok(())
        }

        fn install_file(&self, _: &Path, _: &[u8], _: u32) -> io::Result<()> {
            Ok(())
        }

        fn create_dirs(&self, path: &Path) -> io::Result<()> {
            self.dir(&path.display().to_string());
            Ok(())
        }

        fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
            if self.writes_fail {
                return Err(io::ErrorKind::StorageFull.into());
            }
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), bytes.to_vec());
            Ok(())
        }

        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            let bytes = self
                .files
                .borrow_mut()
                .remove(from)
                .ok_or(io::ErrorKind::NotFound)?;
            self.files.borrow_mut().insert(to.to_path_buf(), bytes);
            Ok(())
        }

        fn remove(&self, path: &Path) {
            self.files.borrow_mut().remove(path);
        }

        fn remove_dir(&self, path: &Path) -> io::Result<()> {
            let busy = self
                .files
                .borrow()
                .keys()
                .chain(self.dirs.borrow().iter())
                .any(|child| child != path && child.starts_with(path));
            if busy || path == Path::new("/boot") {
                return Err(io::ErrorKind::DirectoryNotEmpty.into());
            }
            self.dirs.borrow_mut().remove(path);
            Ok(())
        }
    }

    #[derive(Default)]
    struct Esps {
        contents: HashMap<String, PathBuf>,
        log: RefCell<Vec<String>>,
    }

    impl Esps {
        fn holding(mut self, device: &str, mount: &str) -> Self {
            self.contents
                .insert(device.to_owned(), PathBuf::from(mount));
            self
        }

        fn log(&self) -> Vec<String> {
            self.log.borrow().clone()
        }
    }

    impl Mounts for Esps {
        fn mount_read_only(&self, device: &str, options: &str) -> io::Result<ReadOnlyMount<'_>> {
            assert_eq!(options, "ro,nodev,nosuid,noexec");
            let Some(path) = self.contents.get(device) else {
                self.log.borrow_mut().push(format!("failed {device}"));
                return Err(io::Error::other("wrong fs type"));
            };
            self.log.borrow_mut().push(format!("mount {device}"));
            let unmounted = format!("umount {device}");
            Ok(ReadOnlyMount::new(path.clone(), move |_| {
                self.log.borrow_mut().push(unmounted);
            }))
        }
    }

    fn settings() -> SyncSettings {
        SyncSettings {
            esp_mount: PathBuf::from("/boot"),
        }
    }

    fn synced(machine: &Machine, disk: &Disk, esps: &Esps) -> String {
        let mut out = Vec::new();
        let mut ports = Ports {
            commands: machine,
            files: disk,
            storage: disk,
            mounts: esps,
            out: &mut out,
        };
        sync(&settings(), &mut ports);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn sync_does_nothing_without_a_mounted_esp() {
        let machine = Machine::new(&["/dev/vdb1"]);
        machine.replace(FINDMNT, Answer::exited(1));
        let disk = Disk::new();
        let esps = Esps::default().holding("/dev/vdb1", "/mnt/vdb1");

        let out = synced(&machine, &disk, &esps);

        assert_eq!(
            out,
            "forge dual-boot: no ESP is mounted on /boot, nothing to do.\n"
        );
        assert_eq!(machine.ran(), [FINDMNT]);
        assert!(esps.log().is_empty());
    }

    #[test]
    fn sync_copies_the_boot_manager_of_another_disk_and_marks_it() {
        let machine = Machine::new(&["/dev/vdb1"]);
        let disk = Disk::new();
        disk.file("/mnt/vdb1/EFI/Microsoft/Boot/bootmgfw.efi", b"windows");
        let esps = Esps::default().holding("/dev/vdb1", "/mnt/vdb1");

        let out = synced(&machine, &disk, &esps);

        assert_eq!(
            out,
            "forge dual-boot: copied the Windows boot manager of /dev/vdb1 to /boot/EFI/Microsoft/Boot/bootmgfw.efi, so the menu shows Windows.\n"
        );
        assert_eq!(disk.content(COPY).as_deref(), Some(&b"windows"[..]));
        assert!(disk.content(MARK).is_some());
        assert!(disk.content(&format!("{COPY}.new")).is_none());
        assert_eq!(esps.log(), ["mount /dev/vdb1", "umount /dev/vdb1"]);
        assert!(machine.ran_starting("efibootmgr").is_empty());
    }

    #[test]
    fn an_identical_copy_is_left_as_it_is() {
        let machine = Machine::new(&["/dev/vdb1"]);
        let disk = Disk::new();
        disk.file("/mnt/vdb1/EFI/Microsoft/Boot/bootmgfw.efi", b"windows");
        disk.file(COPY, b"windows");
        disk.file(MARK, b"");

        let out = synced(
            &machine,
            &disk,
            &Esps::default().holding("/dev/vdb1", "/mnt/vdb1"),
        );

        assert_eq!(out, "");
    }

    #[test]
    fn sync_skips_an_esp_that_fails_to_mount() {
        let machine = Machine::new(&["/dev/vdb1", "/dev/vdc1"]);
        let disk = Disk::new();
        disk.file("/mnt/vdc1/EFI/Microsoft/Boot/bootmgfw.efi", b"windows");
        let esps = Esps::default().holding("/dev/vdc1", "/mnt/vdc1");

        let out = synced(&machine, &disk, &esps);

        assert!(
            out.contains("copied the Windows boot manager of /dev/vdc1"),
            "{out}"
        );
        assert_eq!(disk.content(COPY).as_deref(), Some(&b"windows"[..]));
        assert_eq!(
            esps.log(),
            ["failed /dev/vdb1", "mount /dev/vdc1", "umount /dev/vdc1"]
        );
    }

    #[test]
    fn an_esp_without_windows_is_released_and_the_next_one_is_tried() {
        let machine = Machine::new(&["/dev/vdb1", "/dev/vdc1"]);
        let disk = Disk::new();
        disk.file("/mnt/vdc1/EFI/Microsoft/Boot/bootmgfw.efi", b"windows");
        let esps = Esps::default()
            .holding("/dev/vdb1", "/mnt/vdb1")
            .holding("/dev/vdc1", "/mnt/vdc1");

        synced(&machine, &disk, &esps);

        assert_eq!(
            esps.log(),
            [
                "mount /dev/vdb1",
                "umount /dev/vdb1",
                "mount /dev/vdc1",
                "umount /dev/vdc1"
            ]
        );
    }

    #[test]
    fn the_read_only_mount_is_released_when_the_copy_fails() {
        let machine = Machine::new(&["/dev/vdb1"]);
        let disk = Disk {
            writes_fail: true,
            ..Disk::new()
        };
        disk.file("/mnt/vdb1/EFI/Microsoft/Boot/bootmgfw.efi", b"windows");
        let esps = Esps::default().holding("/dev/vdb1", "/mnt/vdb1");

        let out = synced(&machine, &disk, &esps);

        assert_eq!(
            out,
            "forge dual-boot: could not copy the Windows boot manager of /dev/vdb1 to /boot.\n"
        );
        assert_eq!(esps.log(), ["mount /dev/vdb1", "umount /dev/vdb1"]);
        assert!(disk.content(MARK).is_none());
    }

    #[test]
    fn sync_removes_its_copy_when_windows_left() {
        let machine = Machine::new(&[]);
        let disk = Disk::new();
        disk.file(COPY, b"windows");
        disk.file(MARK, b"");

        let out = synced(&machine, &disk, &Esps::default());

        assert_eq!(
            out,
            "forge dual-boot: no other disk holds Windows any more: removed the copy of its boot manager.\n"
        );
        assert!(disk.content(COPY).is_none() && disk.content(MARK).is_none());
        assert!(!disk.has_dir("/boot/EFI/Microsoft/Boot") && !disk.has_dir("/boot/EFI/Microsoft"));
        assert!(disk.has_dir("/boot/EFI/systemd"));
    }

    #[test]
    fn an_esp_that_fails_to_mount_keeps_the_copy() {
        let machine = Machine::new(&["/dev/vdb1"]);
        let disk = Disk::new();
        disk.file(COPY, b"windows");
        disk.file(MARK, b"");

        let out = synced(&machine, &disk, &Esps::default());

        assert_eq!(out, "");
        assert_eq!(disk.content(COPY).as_deref(), Some(&b"windows"[..]));
        assert!(disk.content(MARK).is_some());
    }

    #[test]
    fn sync_never_touches_windows_on_its_own_esp() {
        let machine = Machine::new(&["/dev/vdb1"]);
        let disk = Disk::new();
        disk.file(COPY, b"shared");
        disk.file("/mnt/vdb1/EFI/Microsoft/Boot/bootmgfw.efi", b"other build");
        let esps = Esps::default().holding("/dev/vdb1", "/mnt/vdb1");

        let out = synced(&machine, &disk, &esps);

        assert_eq!(out, "");
        assert_eq!(disk.content(COPY).as_deref(), Some(&b"shared"[..]));
        assert!(disk.content(MARK).is_none());
        assert!(esps.log().is_empty());
    }

    #[test]
    fn partitions_that_cannot_be_listed_leave_the_copy_alone() {
        let machine = Machine::new(&[]);
        machine.replace(LSBLK, Answer::failed("lsblk: boom"));
        let disk = Disk::new();
        disk.file(COPY, b"windows");
        disk.file(MARK, b"");

        let out = synced(&machine, &disk, &Esps::default());

        assert_eq!(
            out,
            "forge dual-boot: cannot list the partitions: lsblk failed: lsblk: boom\n"
        );
        assert!(disk.content(COPY).is_some());
    }

    #[test]
    fn without_efi_variables_the_firmware_is_left_alone() {
        let machine = Machine::new(&["/dev/vdb1"]);
        let disk = Disk::new();
        disk.file(COPY, b"shared");

        synced(&machine, &disk, &Esps::default());

        assert!(machine.ran_starting("efibootmgr").is_empty());
    }

    fn with_efivars() -> Disk {
        let disk = Disk {
            efivars: true,
            ..Disk::new()
        };
        disk.file("/mnt/vdb1/EFI/Microsoft/Boot/bootmgfw.efi", b"windows");
        disk.file(COPY, b"windows");
        disk.file(MARK, b"");
        disk
    }

    #[test]
    fn an_ordered_boot_order_is_left_alone() {
        let machine = Machine::new(&["/dev/vdb1"]);
        machine.answer(
            "efibootmgr",
            Answer::ok(&efibootmgr(
                &[
                    ("0001", "Linux Boot Manager"),
                    ("0003", "Windows Boot Manager"),
                    ("0000", "UEFI Shell"),
                ],
                "0001,0003,0000",
            )),
        );
        let disk = with_efivars();

        let out = synced(
            &machine,
            &disk,
            &Esps::default().holding("/dev/vdb1", "/mnt/vdb1"),
        );

        assert_eq!(out, "");
        assert_eq!(machine.ran_starting("efibootmgr "), Vec::<String>::new());
    }

    #[test]
    fn an_unordered_boot_order_puts_nixos_then_windows_first() {
        let machine = Machine::new(&["/dev/vdb1"]);
        machine.answer(
            "efibootmgr",
            Answer::ok(&efibootmgr(
                &[
                    ("0001", "Linux Boot Manager"),
                    ("0003", "Windows Boot Manager"),
                    ("0000", "UEFI Shell"),
                ],
                "0000,0003,0004,0001",
            )),
        );
        let disk = with_efivars();

        let out = synced(
            &machine,
            &disk,
            &Esps::default().holding("/dev/vdb1", "/mnt/vdb1"),
        );

        assert_eq!(
            out,
            "forge dual-boot: firmware boot order set to 0001,0003,0000,0004, NixOS first and Windows next.\n"
        );
        assert_eq!(
            machine.ran_starting("efibootmgr "),
            ["efibootmgr --bootorder 0001,0003,0000,0004"]
        );
    }

    #[test]
    fn a_missing_windows_entry_is_created_on_the_esp_partition() {
        let machine = Machine::new(&["/dev/vdb1"]);
        let without = efibootmgr(
            &[("0001", "Linux Boot Manager"), ("0000", "UEFI Shell")],
            "0001,0000",
        );
        let with = efibootmgr(
            &[
                ("0001", "Linux Boot Manager"),
                ("0000", "UEFI Shell"),
                ("0003", "Windows Boot Manager"),
            ],
            "0001,0000",
        );
        machine.answer("efibootmgr", Answer::ok(&without));
        machine.answer("efibootmgr", Answer::ok(&without));
        machine.answer("efibootmgr", Answer::ok(&with));
        let disk = with_efivars();

        let out = synced(
            &machine,
            &disk,
            &Esps::default().holding("/dev/vdb1", "/mnt/vdb1"),
        );

        assert_eq!(
            out,
            "forge dual-boot: created the firmware entry Windows Boot Manager.\nforge dual-boot: firmware boot order set to 0001,0003,0000, NixOS first and Windows next.\n"
        );
        assert_eq!(
            machine.ran_starting("efibootmgr "),
            [CREATE_WINDOWS, "efibootmgr --bootorder 0001,0003,0000"]
        );
    }

    #[test]
    fn windows_on_its_own_esp_gets_its_entry_on_that_esp() {
        let machine = Machine::new(&[]);
        let without = efibootmgr(&[("0001", "Linux Boot Manager")], "0001");
        let with = efibootmgr(
            &[
                ("0001", "Linux Boot Manager"),
                ("0002", "Windows Boot Manager"),
            ],
            "0001,0002",
        );
        machine.answer("efibootmgr", Answer::ok(&without));
        machine.answer("efibootmgr", Answer::ok(&without));
        machine.answer("efibootmgr", Answer::ok(&with));
        let disk = Disk {
            efivars: true,
            ..Disk::new()
        };
        disk.file(COPY, b"shared");

        let out = synced(&machine, &disk, &Esps::default());

        assert_eq!(
            out,
            "forge dual-boot: created the firmware entry Windows Boot Manager.\n"
        );
        assert_eq!(
            machine.ran_starting("efibootmgr "),
            [
                "efibootmgr --create-only --disk /dev/vda --part 1 --label Windows Boot Manager --loader \\EFI\\Microsoft\\Boot\\bootmgfw.efi"
            ]
        );
    }

    #[test]
    fn an_inactive_windows_entry_is_reused_not_duplicated() {
        let machine = Machine::new(&["/dev/vdb1"]);
        machine.answer(
            "efibootmgr",
            Answer::ok(
                "BootCurrent: 0001\nBootOrder: 0001,0003\nBoot0001* Linux Boot Manager\tHD(1,GPT,abc)/File(\\EFI\\x.efi)\nBoot0003  Windows Boot Manager\tHD(1,GPT,def)/File(\\EFI\\y.efi)\n",
            ),
        );
        let disk = with_efivars();

        let out = synced(
            &machine,
            &disk,
            &Esps::default().holding("/dev/vdb1", "/mnt/vdb1"),
        );

        assert_eq!(out, "");
        assert_eq!(machine.ran_starting("efibootmgr "), Vec::<String>::new());
    }

    #[test]
    fn a_failed_entry_creation_warns_and_leaves_the_order() {
        let machine = Machine::new(&["/dev/vdb1"]);
        machine.answer(
            "efibootmgr",
            Answer::ok(&efibootmgr(&[("0001", "Linux Boot Manager")], "0001")),
        );
        machine.answer(CREATE_WINDOWS, Answer::exited(5));
        let disk = with_efivars();

        let out = synced(
            &machine,
            &disk,
            &Esps::default().holding("/dev/vdb1", "/mnt/vdb1"),
        );

        assert_eq!(
            out,
            "forge dual-boot: could not set the firmware boot entries: pick Windows in the menu once its firmware entry exists.\n"
        );
        assert_eq!(machine.ran_starting("efibootmgr "), [CREATE_WINDOWS]);
    }
}
