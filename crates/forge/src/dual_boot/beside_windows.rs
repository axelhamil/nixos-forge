use std::io::Write;
use std::path::Path;
use std::time::Duration;

use super::config::BesideWindowsSettings;
use crate::system::interrupt::Flag;
use crate::system::mount::Mounts;
use crate::system::partitions::{self, Partition};
use crate::system::{Answer, Commands, Files, Invocation, Sizes, Storage};

const MIB: u64 = 1024 * 1024;
const GENERATIONS: u64 = 3;
const MARGIN: u64 = 32 * MIB;
const STORE_MARGIN: u64 = 1024 * MIB;
const BOOT_MANAGER: &str = "EFI/Microsoft/Boot/bootmgfw.efi";
const REUSED_ON_THE_ESP: [&str; 3] = ["EFI/nixos", "EFI/Linux", "loader"];
const LUKS_TYPE_CODE: &str = "8309";
const SETTLE_TIMEOUT: Duration = Duration::from_secs(150);
const REFUSED: u8 = 1;
const SIGINT: i32 = 2;
const INTERRUPTED: u8 = 130;
const TERMINATED: u8 = 143;

pub struct Ports<'a> {
    pub commands: &'a dyn Commands,
    pub files: &'a dyn Files,
    pub storage: &'a dyn Storage,
    pub mounts: &'a dyn Mounts,
    pub sizes: &'a dyn Sizes,
    pub interrupt: &'a Flag,
    pub out: &'a mut dyn Write,
}

impl Ports<'_> {
    fn say(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }

    fn checkpoint(&self) -> Result<(), Stop> {
        match self.interrupt.raised() {
            Some(signal) => Err(Stop::Interrupted(signal)),
            None => Ok(()),
        }
    }
}

enum Stop {
    Refused(String),
    Interrupted(i32),
}

pub struct Labels {
    pub esp: String,
    luks: String,
}

impl Labels {
    pub fn of(settings: &BesideWindowsSettings) -> Self {
        let last = |path: &Path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        Self {
            esp: last(&settings.esp_device),
            luks: last(&settings.luks_device),
        }
    }
}

pub fn install(
    settings: &BesideWindowsSettings,
    disk: &str,
    system: &str,
    ports: &mut Ports,
) -> u8 {
    let labels = Labels::of(settings);
    match steps(settings, &labels, disk, system, ports) {
        Ok(code) => code,
        Err(Stop::Refused(message)) => {
            let _ = writeln!(ports.out, "\nforge dual-boot: {message}");
            REFUSED
        }
        Err(Stop::Interrupted(SIGINT)) => INTERRUPTED,
        Err(Stop::Interrupted(_)) => TERMINATED,
    }
}

#[must_use]
pub fn usage(esp_label: Option<&str>) -> String {
    let renamed = esp_label.map_or_else(String::new, |label| format!("to {label}, "));
    format!(
        "Usage: forge dual-boot install-beside-windows DISK SYSTEM

Creates the encrypted NixOS partition in the free space of DISK, which holds
Windows, and mounts it on /mnt with the Windows ESP on /mnt/boot. The Windows
partitions and their content are kept; only the GPT name of the ESP changes,
{renamed}so the installed system finds it.
SYSTEM is the system to install: it sets the room needed on the ESP and the disk.
"
    )
}

fn steps(
    settings: &BesideWindowsSettings,
    labels: &Labels,
    disk_arg: &str,
    system: &str,
    ports: &mut Ports,
) -> Result<u8, Stop> {
    let disk = ports
        .files
        .canonicalize(Path::new(disk_arg))
        .display()
        .to_string();
    if !ports.storage.is_block_device(Path::new(&disk)) {
        return Err(Stop::Refused(format!("{disk_arg} is not a block device.")));
    }
    let kernel = Path::new(system).join("kernel");
    let initrd = Path::new(system).join("initrd");
    if !ports.storage.exists(&kernel) || !ports.storage.exists(&initrd) {
        return Err(Stop::Refused(format!("{system} is not a NixOS system.")));
    }

    let partitions = list_partitions(ports, &disk)?;
    if let Some(parent) = partitions
        .iter()
        .find_map(|partition| partition.disk.as_deref().filter(|parent| *parent != disk))
    {
        return Err(Stop::Refused(format!(
            "{disk} is a partition of {parent}: name the whole disk that holds Windows."
        )));
    }
    let esp = only_esp(&disk, &partitions)?;
    let generation_size = size_of(ports, &kernel)? + size_of(ports, &initrd)?;
    check_esp_room(ports, &disk, &esp.name, generation_size)?;

    match partitions
        .iter()
        .find(|partition| partition.label.as_deref() == Some(labels.luks.as_str()))
    {
        Some(luks) => ports.say(&format!(
            "Reusing {}, the NixOS partition of a previous install: its content is erased.",
            luks.name
        )),
        None => create_partition(ports, &disk, system, &labels.luks)?,
    }

    if esp.label.as_deref() != Some(labels.esp.as_str()) {
        let number = esp.number.ok_or_else(|| {
            Stop::Refused(format!("cannot read the partition number of {}.", esp.name))
        })?;
        succeed(
            ports,
            &Invocation::new(
                "sgdisk",
                &[&format!("--change-name={number}:{}", labels.esp), &disk],
            ),
        )?;
        ports.say(&format!(
            "Named the ESP {} {} in the partition table.",
            esp.name, labels.esp
        ));
    }

    succeed(ports, &Invocation::new("partprobe", &[&disk]))?;
    succeed(
        ports,
        &Invocation::new("udevadm", &["settle"]).timeout(SETTLE_TIMEOUT),
    )?;
    if !ports.storage.exists(&settings.luks_device) || !ports.storage.exists(&settings.esp_device) {
        return Err(Stop::Refused(
            "the kernel does not show the new partition labels yet: reboot the installer and run again.".to_owned(),
        ));
    }
    let relisted = list_partitions(ports, &disk)?;
    for link in [&settings.luks_device, &settings.esp_device] {
        on_disk(ports, link, &disk, &relisted)?;
    }
    let luks_device = settings.luks_device.display().to_string();
    succeed(ports, &Invocation::new("wipefs", &["-a", &luks_device]))?;

    let formatted = ports
        .commands
        .run(&Invocation::new(&settings.format_mount, &[]).attached());
    ports.checkpoint()?;
    Ok(formatted.exit_code())
}

fn list_partitions(ports: &Ports, disk: &str) -> Result<Vec<Partition>, Stop> {
    let listed = partitions::list(ports.commands, Some(disk));
    ports.checkpoint()?;
    listed.map_err(|error| Stop::Refused(format!("cannot list the partitions of {disk}: {error}")))
}

fn on_disk(ports: &Ports, link: &Path, disk: &str, partitions: &[Partition]) -> Result<(), Stop> {
    let device = ports.files.canonicalize(link).display().to_string();
    if partitions
        .iter()
        .any(|partition| partition.name == device && partition.disk.as_deref() == Some(disk))
    {
        return Ok(());
    }
    Err(Stop::Refused(format!(
        "{} points to {device}, not a partition of {disk}: remove the old forge labels on the other disk, then run again.",
        link.display()
    )))
}

fn only_esp<'p>(disk: &str, partitions: &'p [Partition]) -> Result<&'p Partition, Stop> {
    let esps: Vec<&Partition> = partitions
        .iter()
        .filter(|partition| partition.is_esp())
        .collect();
    match esps.as_slice() {
        [esp] => Ok(esp),
        [] => Err(Stop::Refused(format!(
            "{disk} has no EFI system partition, so no Windows to install beside. Install on a whole disk instead, or check the disk name."
        ))),
        _ => Err(Stop::Refused(format!(
            "{disk} has {} EFI system partitions ({}): keep one, then run again.",
            esps.len(),
            esps.iter()
                .map(|esp| esp.name.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ))),
    }
}

fn size_of(ports: &Ports, path: &Path) -> Result<u64, Stop> {
    ports.sizes.file_size(path).map_err(|error| {
        Stop::Refused(format!(
            "cannot read the size of {}: {error}",
            path.display()
        ))
    })
}

fn check_esp_room(ports: &Ports, disk: &str, esp: &str, generation_size: u64) -> Result<(), Stop> {
    let mounted = ports.mounts.mount_read_only(esp, "ro");
    ports.checkpoint()?;
    let mounted = mounted.map_err(|_| Stop::Refused(format!("cannot mount the ESP {esp}.")))?;
    if !ports.storage.exists(&mounted.path().join(BOOT_MANAGER)) {
        return Err(Stop::Refused(format!(
            "the ESP {esp} holds no {BOOT_MANAGER}: Windows is not installed on {disk}."
        )));
    }
    let available = ports.sizes.available(mounted.path()).map_err(|error| {
        Stop::Refused(format!(
            "cannot read the free space of the ESP {esp}: {error}"
        ))
    })?;
    let reused: u64 = REUSED_ON_THE_ESP
        .iter()
        .map(|dir| ports.sizes.tree_size(&mounted.path().join(dir)))
        .sum();
    ports.checkpoint()?;
    drop(mounted);

    let needed = GENERATIONS * generation_size + MARGIN;
    let free = available.saturating_add(reused);
    if free >= needed {
        return Ok(());
    }
    Err(Stop::Refused(format!(
        "the ESP {esp} has {} MiB free, and this system needs {} MiB to keep {GENERATIONS} generations ({} MiB each).
Grow the ESP to 1 GiB first: shrink the Windows partition in Windows Disk Management, then move it and grow the ESP with GParted from a live USB.
Otherwise install NixOS on its own disk, which gets its own ESP.",
        free / MIB,
        needed / MIB,
        generation_size / MIB
    )))
}

fn create_partition(
    ports: &mut Ports,
    disk: &str,
    system: &str,
    luks_label: &str,
) -> Result<(), Stop> {
    let needed_space = closure_size(ports, system)? + STORE_MARGIN;
    let start = last_number(
        ports,
        &Invocation::new("sgdisk", &["--first-aligned-in-largest", disk]),
    )?;
    let end = last_number(
        ports,
        &Invocation::new("sgdisk", &["--end-of-largest", disk]),
    )?;
    let sector = last_number(ports, &Invocation::new("blockdev", &["--getss", disk]))?;
    let largest = if end >= start {
        (end - start + 1).saturating_mul(sector)
    } else {
        0
    };
    if start >= end || largest < needed_space {
        return Err(Stop::Refused(format!(
            "{disk} has {} MiB of free space in one block, and NixOS needs at least {} MiB.
Shrink the Windows partition in Windows Disk Management (Shrink Volume), then run again.",
            largest / MIB,
            needed_space / MIB
        )));
    }

    let printed = succeed(ports, &Invocation::new("sgdisk", &["--print", disk]))?;
    let number = printed
        .stdout
        .lines()
        .rev()
        .find_map(|line| line.split_whitespace().next()?.parse::<u32>().ok())
        .unwrap_or_default()
        + 1;
    succeed(
        ports,
        &Invocation::new(
            "sgdisk",
            &[
                &format!("--new={number}:{start}:{end}"),
                &format!("--change-name={number}:{luks_label}"),
                &format!("--typecode={number}:{LUKS_TYPE_CODE}"),
                disk,
            ],
        ),
    )?;
    ports.say(&format!(
        "Created partition {number} of {disk}, {} MiB, for NixOS.",
        largest / MIB
    ));
    Ok(())
}

fn closure_size(ports: &Ports, system: &str) -> Result<u64, Stop> {
    let requisites = succeed(
        ports,
        &Invocation::new("nix-store", &["--query", "--requisites", system]),
    )?;
    let paths: Vec<&str> = requisites.stdout.split_whitespace().collect();
    let args: Vec<&str> = ["--query", "--size"].into_iter().chain(paths).collect();
    let sizes = succeed(ports, &Invocation::new("nix-store", &args))?;
    Ok(sizes
        .stdout
        .split_whitespace()
        .filter_map(|size| size.parse::<u64>().ok())
        .sum())
}

fn last_number(ports: &Ports, invocation: &Invocation) -> Result<u64, Stop> {
    let answer = succeed(ports, invocation)?;
    answer
        .stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| line.trim().parse().ok())
        .ok_or_else(|| {
            Stop::Refused(format!(
                "cannot read a number from {}: {}",
                invocation.command_line(),
                answer.stdout.trim()
            ))
        })
}

fn succeed(ports: &Ports, invocation: &Invocation) -> Result<Answer, Stop> {
    let answer = ports.commands.run(invocation);
    ports.checkpoint()?;
    if answer.success {
        return Ok(answer);
    }
    Err(Stop::Refused(format!(
        "{} failed: {}",
        invocation.command_line(),
        answer.stderr.trim()
    )))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::io;
    use std::path::{Path, PathBuf};

    use std::time::Duration;

    use super::*;
    use crate::system::interrupt::Flag;
    use crate::system::mount::ReadOnlyMount;
    use crate::system::{Answer, Output, Owner};

    const MIB: u64 = 1024 * 1024;
    const LSBLK: &str = "lsblk -J -l -p -o NAME,TYPE,PKNAME,PARTN,PARTTYPE,PARTLABEL /dev/vdb";
    const ESP_TYPE: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
    const LUKS_TYPE: &str = "ca7d7ccb-63ed-4c53-861c-1742536059cc";
    const BASIC_DATA: &str = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";
    const BOOT_MANAGER: &str = "/run/esp/EFI/Microsoft/Boot/bootmgfw.efi";
    const LUKS: &str = "/dev/disk/by-partlabel/disk-main-luks";
    const ESP: &str = "/dev/disk/by-partlabel/disk-main-ESP";
    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;
    const CREATE: &str =
        "sgdisk --new=4:1050624:25163742 --change-name=4:disk-main-luks --typecode=4:8309 /dev/vdb";

    fn row(number: u32, part_type: &str, label: &str) -> String {
        format!(
            r#"{{"name":"/dev/vdb{number}","type":"part","pkname":"/dev/vdb","partn":{number},"parttype":"{part_type}","partlabel":"{label}"}}"#
        )
    }

    fn lsblk(rows: &[String]) -> String {
        let disk = r#"{"name":"/dev/vdb","type":"disk","pkname":null,"partn":null,"parttype":null,"partlabel":null}"#;
        let rows = std::iter::once(disk.to_owned())
            .chain(rows.iter().cloned())
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"blockdevices": [{rows}]}}"#)
    }

    fn windows_rows() -> Vec<String> {
        vec![
            row(1, ESP_TYPE, "EFI system partition"),
            row(
                2,
                "e3c9e316-0b5c-4db8-817d-f92df00215ae",
                "Microsoft reserved partition",
            ),
            row(3, BASIC_DATA, "Basic data partition"),
        ]
    }

    struct Windows {
        answers: RefCell<HashMap<String, VecDeque<Answer>>>,
        log: RefCell<Vec<String>>,
        paths: RefCell<HashSet<PathBuf>>,
        links: RefCell<HashMap<PathBuf, PathBuf>>,
        sizes: HashMap<PathBuf, u64>,
        available: u64,
        interrupt: Flag,
        raise_on: RefCell<Option<(String, i32)>>,
    }

    impl Windows {
        fn new() -> Self {
            let windows = Self {
                answers: RefCell::new(HashMap::new()),
                log: RefCell::new(Vec::new()),
                paths: RefCell::new(HashSet::new()),
                links: RefCell::new(HashMap::from([
                    (
                        PathBuf::from("/dev/disk/by-id/windows"),
                        PathBuf::from("/dev/vdb"),
                    ),
                    (PathBuf::from(LUKS), PathBuf::from("/dev/vdb4")),
                    (PathBuf::from(ESP), PathBuf::from("/dev/vdb1")),
                ])),
                sizes: HashMap::from([
                    (PathBuf::from("/nix/store/system/kernel"), 20 * MIB),
                    (PathBuf::from("/nix/store/system/initrd"), 30 * MIB),
                ]),
                available: 400 * MIB,
                interrupt: Flag::default(),
                raise_on: RefCell::new(None),
            };
            for path in [
                "/dev/vdb",
                "/nix/store/system/kernel",
                "/nix/store/system/initrd",
                BOOT_MANAGER,
                LUKS,
                ESP,
            ] {
                windows.paths.borrow_mut().insert(PathBuf::from(path));
            }
            windows.answer(LSBLK, Answer::ok(&lsblk(&windows_rows())));
            let mut created = windows_rows();
            created.push(row(4, LUKS_TYPE, "disk-main-luks"));
            windows.then(LSBLK, Answer::ok(&lsblk(&created)));
            windows.answer(
                "nix-store --query --requisites /nix/store/system",
                Answer::ok("/nix/store/a\n/nix/store/b\n"),
            );
            windows.answer(
                "nix-store --query --size /nix/store/a /nix/store/b",
                Answer::ok("1048576\n2097152\n"),
            );
            windows.answer(
                "sgdisk --first-aligned-in-largest /dev/vdb",
                Answer::ok("Warning: a note\n1050624\n"),
            );
            windows.answer("sgdisk --end-of-largest /dev/vdb", Answer::ok("25163742\n"));
            windows.answer("blockdev --getss /dev/vdb", Answer::ok("512\n"));
            windows.answer(
                "sgdisk --print /dev/vdb",
                Answer::ok(
                    "Disk /dev/vdb: 25165824 sectors, 12.0 GiB\nNumber  Start (sector)    End (sector)  Size       Code  Name\n   1            2048          206847   100.0 MiB   EF00  EFI system partition\n   2          206848          239615   16.0 MiB    0C01  Microsoft reserved partition\n   3          239616         1050623   396.0 MiB   0700  Basic data partition\n",
                ),
            );
            windows
        }

        fn answer(&self, command: &str, answer: Answer) {
            self.answers
                .borrow_mut()
                .insert(command.to_owned(), VecDeque::from([answer]));
        }

        fn then(&self, command: &str, answer: Answer) {
            self.answers
                .borrow_mut()
                .entry(command.to_owned())
                .or_default()
                .push_back(answer);
        }

        fn link(&self, link: &str, device: &str) {
            self.links
                .borrow_mut()
                .insert(PathBuf::from(link), PathBuf::from(device));
        }

        fn raise_on(&self, line: &str, signal: i32) {
            *self.raise_on.borrow_mut() = Some((line.to_owned(), signal));
        }

        fn logged(&self, line: String) {
            if let Some((watched, signal)) = self.raise_on.borrow().as_ref()
                && *watched == line
            {
                self.interrupt.raise(*signal);
            }
            self.log.borrow_mut().push(line);
        }

        fn without(&self, path: &str) {
            self.paths.borrow_mut().remove(Path::new(path));
        }

        fn ran(&self) -> Vec<String> {
            self.log.borrow().clone()
        }

        fn wrote(&self) -> bool {
            self.ran().iter().any(|line| {
                line.starts_with("sgdisk --new")
                    || line.starts_with("sgdisk --change-name")
                    || line.starts_with("wipefs")
                    || line.starts_with("/nix/store/disko")
            })
        }
    }

    impl Commands for Windows {
        fn run(&self, invocation: &Invocation) -> Answer {
            let line = invocation.command_line();
            if invocation.program == "/nix/store/disko" {
                assert_eq!(invocation.output, Output::Attached);
            }
            if invocation.program == "udevadm" {
                assert_eq!(invocation.timeout, Some(Duration::from_secs(150)));
            }
            self.logged(line.clone());
            let mut answers = self.answers.borrow_mut();
            let Some(queue) = answers.get_mut(&line) else {
                return Answer::ok("");
            };
            if queue.len() > 1 {
                return queue.pop_front().unwrap_or_default();
            }
            queue.front().cloned().unwrap_or_default()
        }
    }

    impl Files for Windows {
        fn read(&self, _: &Path) -> io::Result<Vec<u8>> {
            Err(io::ErrorKind::NotFound.into())
        }

        fn read_link(&self, _: &Path) -> Option<PathBuf> {
            None
        }

        fn is_dir(&self, _: &Path) -> bool {
            false
        }

        fn list(&self, _: &Path) -> Vec<String> {
            Vec::new()
        }

        fn canonicalize(&self, path: &Path) -> PathBuf {
            self.links
                .borrow()
                .get(path)
                .cloned()
                .unwrap_or_else(|| path.to_path_buf())
        }

        fn owner(&self, _: &Path) -> Option<Owner> {
            None
        }
    }

    impl Storage for Windows {
        fn is_block_device(&self, path: &Path) -> bool {
            path == Path::new("/dev/vdb") || path == Path::new("/dev/vdb1")
        }

        fn exists(&self, path: &Path) -> bool {
            self.paths.borrow().contains(path)
        }

        fn link(&self, _: &Path, _: &Path) -> io::Result<()> {
            Ok(())
        }

        fn install_dir(&self, _: &Path, _: u32) -> io::Result<()> {
            Ok(())
        }

        fn install_file(&self, _: &Path, _: &[u8], _: u32) -> io::Result<()> {
            Ok(())
        }

        fn create_dirs(&self, _: &Path) -> io::Result<()> {
            Ok(())
        }

        fn write(&self, _: &Path, _: &[u8]) -> io::Result<()> {
            Ok(())
        }

        fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
            Ok(())
        }

        fn remove(&self, _: &Path) {}

        fn remove_dir(&self, _: &Path) -> io::Result<()> {
            Ok(())
        }
    }

    impl Mounts for Windows {
        fn mount_read_only(&self, device: &str, options: &str) -> io::Result<ReadOnlyMount<'_>> {
            self.logged(format!("mount -o {options} {device}"));
            Ok(ReadOnlyMount::new(PathBuf::from("/run/esp"), |_| {
                self.log.borrow_mut().push("umount /run/esp".to_owned());
            }))
        }
    }

    impl Sizes for Windows {
        fn available(&self, path: &Path) -> io::Result<u64> {
            assert_eq!(path, Path::new("/run/esp"));
            Ok(self.available)
        }

        fn file_size(&self, path: &Path) -> io::Result<u64> {
            self.sizes
                .get(path)
                .copied()
                .ok_or_else(|| io::ErrorKind::NotFound.into())
        }

        fn tree_size(&self, path: &Path) -> u64 {
            self.sizes.get(path).copied().unwrap_or_default()
        }
    }

    fn settings() -> BesideWindowsSettings {
        BesideWindowsSettings {
            format_mount: "/nix/store/disko".to_owned(),
            esp_device: PathBuf::from(ESP),
            luks_device: PathBuf::from(LUKS),
        }
    }

    fn installed(windows: &Windows, disk: &str) -> (u8, String) {
        let mut out = Vec::new();
        let mut ports = Ports {
            commands: windows,
            files: windows,
            storage: windows,
            mounts: windows,
            sizes: windows,
            interrupt: &windows.interrupt,
            out: &mut out,
        };
        let code = install(&settings(), disk, "/nix/store/system", &mut ports);
        (code, String::from_utf8(out).unwrap())
    }

    fn refused(windows: &Windows) -> String {
        let (code, out) = installed(windows, "/dev/vdb");
        assert_eq!(code, 1, "{out}");
        assert!(!windows.wrote(), "{:?}", windows.ran());
        out
    }

    #[test]
    fn the_usage_names_the_esp_label_only_when_it_is_known() {
        assert!(
            usage(Some("disk-main-ESP"))
                .starts_with("Usage: forge dual-boot install-beside-windows DISK SYSTEM\n")
        );
        assert!(
            usage(Some("disk-main-ESP"))
                .contains("only the GPT name of the ESP changes,\nto disk-main-ESP, so")
        );
        assert!(usage(None).contains("only the GPT name of the ESP changes,\nso the installed"));
    }

    #[test]
    fn a_partition_given_as_the_disk_is_refused() {
        let windows = Windows::new();
        windows.answer(
            "lsblk -J -l -p -o NAME,TYPE,PKNAME,PARTN,PARTTYPE,PARTLABEL /dev/vdb1",
            Answer::ok(&format!(
                r#"{{"blockdevices": [{}]}}"#,
                row(1, ESP_TYPE, "EFI system partition")
            )),
        );

        let (code, out) = installed(&windows, "/dev/vdb1");

        assert_eq!(code, 1);
        assert_eq!(
            out,
            "\nforge dual-boot: /dev/vdb1 is a partition of /dev/vdb: name the whole disk that holds Windows.\n"
        );
        assert!(!windows.wrote(), "{:?}", windows.ran());
        assert!(!windows.ran().iter().any(|line| line.starts_with("mount")));
    }

    #[test]
    fn a_label_that_points_to_another_disk_stops_before_the_wipe() {
        let windows = Windows::new();
        windows.link(ESP, "/dev/vdc1");

        let (code, out) = installed(&windows, "/dev/vdb");

        assert_eq!(code, 1);
        assert!(
            out.ends_with("\nforge dual-boot: /dev/disk/by-partlabel/disk-main-ESP points to /dev/vdc1, not a partition of /dev/vdb: remove the old forge labels on the other disk, then run again.\n"),
            "{out}"
        );
        assert!(
            !windows
                .ran()
                .iter()
                .any(|line| line.starts_with("wipefs") || line == "/nix/store/disko")
        );
    }

    #[test]
    fn an_interrupt_during_the_esp_inspection_unmounts_it_and_exits_130() {
        let windows = Windows::new();
        windows.raise_on("mount -o ro /dev/vdb1", SIGINT);

        let (code, _) = installed(&windows, "/dev/vdb");

        assert_eq!(code, 130);
        assert_eq!(
            windows.ran(),
            [LSBLK, "mount -o ro /dev/vdb1", "umount /run/esp"]
        );
    }

    #[test]
    fn a_stop_request_before_the_wipe_exits_143_without_wiping() {
        let windows = Windows::new();
        windows.raise_on("partprobe /dev/vdb", SIGTERM);

        let (code, _) = installed(&windows, "/dev/vdb");

        assert_eq!(code, 143);
        assert_eq!(
            windows.ran().last().map(String::as_str),
            Some("partprobe /dev/vdb")
        );
    }

    #[test]
    fn a_disk_that_is_not_a_block_device_or_a_system_that_is_not_nixos_is_refused() {
        let windows = Windows::new();
        let (code, out) = installed(&windows, "/dev/vdz");
        assert_eq!(code, 1);
        assert_eq!(out, "\nforge dual-boot: /dev/vdz is not a block device.\n");

        windows.without("/nix/store/system/initrd");
        assert_eq!(
            refused(&windows),
            "\nforge dual-boot: /nix/store/system is not a NixOS system.\n"
        );
    }

    #[test]
    fn a_disk_without_an_esp_is_refused() {
        let windows = Windows::new();
        windows.answer(LSBLK, Answer::ok(&lsblk(&[row(1, BASIC_DATA, "data")])));

        assert_eq!(
            refused(&windows),
            "\nforge dual-boot: /dev/vdb has no EFI system partition, so no Windows to install beside. Install on a whole disk instead, or check the disk name.\n"
        );
    }

    #[test]
    fn a_disk_with_two_esps_is_refused() {
        let windows = Windows::new();
        let mut rows = windows_rows();
        rows.push(row(4, &ESP_TYPE.to_lowercase(), "second"));
        windows.answer(LSBLK, Answer::ok(&lsblk(&rows)));

        assert_eq!(
            refused(&windows),
            "\nforge dual-boot: /dev/vdb has 2 EFI system partitions (/dev/vdb1 /dev/vdb4): keep one, then run again.\n"
        );
    }

    #[test]
    fn an_esp_without_the_windows_boot_manager_is_refused_and_unmounted() {
        let windows = Windows::new();
        windows.without(BOOT_MANAGER);

        assert_eq!(
            refused(&windows),
            "\nforge dual-boot: the ESP /dev/vdb1 holds no EFI/Microsoft/Boot/bootmgfw.efi: Windows is not installed on /dev/vdb.\n"
        );
        assert_eq!(
            windows.ran(),
            [LSBLK, "mount -o ro /dev/vdb1", "umount /run/esp"]
        );
    }

    #[test]
    fn an_esp_too_small_for_three_generations_is_refused() {
        let windows = Windows {
            available: 100 * MIB,
            ..Windows::new()
        };

        assert_eq!(
            refused(&windows),
            "\nforge dual-boot: the ESP /dev/vdb1 has 100 MiB free, and this system needs 182 MiB to keep 3 generations (50 MiB each).
Grow the ESP to 1 GiB first: shrink the Windows partition in Windows Disk Management, then move it and grow the ESP with GParted from a live USB.
Otherwise install NixOS on its own disk, which gets its own ESP.\n"
        );
    }

    #[test]
    fn a_disk_with_too_little_free_space_is_refused() {
        let windows = Windows::new();
        windows.answer("sgdisk --end-of-largest /dev/vdb", Answer::ok("1060000\n"));

        assert_eq!(
            refused(&windows),
            "\nforge dual-boot: /dev/vdb has 4 MiB of free space in one block, and NixOS needs at least 1027 MiB.
Shrink the Windows partition in Windows Disk Management (Shrink Volume), then run again.\n"
        );
    }

    #[test]
    fn reuses_the_partition_of_a_previous_install() {
        let mut windows = Windows {
            available: 150 * MIB,
            ..Windows::new()
        };
        windows
            .sizes
            .insert(PathBuf::from("/run/esp/EFI/nixos"), 40 * MIB);
        let mut rows = windows_rows();
        rows[0] = row(1, ESP_TYPE, "disk-main-ESP");
        rows.push(row(4, LUKS_TYPE, "disk-main-luks"));
        windows.answer(LSBLK, Answer::ok(&lsblk(&rows)));

        let (code, out) = installed(&windows, "/dev/vdb");

        assert_eq!(code, 0, "{out}");
        assert_eq!(
            out,
            "Reusing /dev/vdb4, the NixOS partition of a previous install: its content is erased.\n"
        );
        assert_eq!(
            windows.ran(),
            [
                LSBLK,
                "mount -o ro /dev/vdb1",
                "umount /run/esp",
                "partprobe /dev/vdb",
                "udevadm settle",
                LSBLK,
                "wipefs -a /dev/disk/by-partlabel/disk-main-luks",
                "/nix/store/disko",
            ]
        );
    }

    #[test]
    fn creates_a_partition_in_the_largest_free_block_and_names_the_esp() {
        let windows = Windows::new();

        let (code, out) = installed(&windows, "/dev/disk/by-id/windows");

        assert_eq!(code, 0, "{out}");
        assert_eq!(
            out,
            "Created partition 4 of /dev/vdb, 11773 MiB, for NixOS.\nNamed the ESP /dev/vdb1 disk-main-ESP in the partition table.\n"
        );
        let ran = windows.ran();
        let position = |command: &str| ran.iter().position(|line| line == command);
        let created = position(CREATE).unwrap();
        let named = position("sgdisk --change-name=1:disk-main-ESP /dev/vdb").unwrap();
        let wiped = position("wipefs -a /dev/disk/by-partlabel/disk-main-luks").unwrap();
        assert!(created < named && named < wiped);
        assert_eq!(ran.last().map(String::as_str), Some("/nix/store/disko"));
    }

    #[test]
    fn labels_the_kernel_does_not_show_yet_stop_before_the_wipe() {
        let windows = Windows::new();
        windows.without(LUKS);

        let (code, out) = installed(&windows, "/dev/vdb");

        assert_eq!(code, 1);
        assert!(out.ends_with("\nforge dual-boot: the kernel does not show the new partition labels yet: reboot the installer and run again.\n"));
        assert!(!windows.ran().iter().any(|line| line.starts_with("wipefs")));
    }

    #[test]
    fn the_exit_code_of_the_format_mount_is_returned() {
        let windows = Windows::new();
        windows.answer("/nix/store/disko", Answer::exited(3));

        let (code, _) = installed(&windows, "/dev/vdb");

        assert_eq!(code, 3);
    }
}
