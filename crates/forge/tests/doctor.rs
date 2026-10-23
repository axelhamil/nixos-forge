use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use forge::doctor::{
    self, Answer, Clock, Commands, Config, Files, Forks, Host, Invocation, Mount, Options, Owner,
    Space, System, Totals,
};
use insta::assert_snapshot;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use tempfile::TempDir;

const NOW: &str = "2026-10-23T12:00:00Z";
const EFIVARS: &str = "/sys/firmware/efi/efivars";
const EFI_GLOBAL: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";
const STATE: &str = "/var/lib/auto-upgrade";
const FLAKE: &str = "/srv/nixos";
const LUKS: &str = "/dev/disk/by-partlabel/disk-main-luks";
const BOOT_SETUP: &str = "systemctl show --timestamp=us+utc -p Id,InactiveExitTimestamp,InactiveExitTimestampMonotonic,ActiveEnterTimestamp,ActiveEnterTimestampMonotonic systemd-tmpfiles-setup.service sysinit.target";
const SETPRIV: &str = "as 1000:100";

struct Fake {
    commands: HashMap<String, Answer>,
    files: HashMap<PathBuf, Vec<u8>>,
    dirs: HashSet<PathBuf>,
    links: HashMap<PathBuf, PathBuf>,
    owners: HashMap<PathBuf, Owner>,
    uid: u32,
    sudo_user: Option<String>,
    space: Option<Space>,
    mounts: Option<Vec<Mount>>,
    now: Timestamp,
    root: PathBuf,
    fork_dirs: Vec<PathBuf>,
    fork_timeout: Duration,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            commands: HashMap::new(),
            files: HashMap::new(),
            dirs: HashSet::new(),
            links: HashMap::new(),
            owners: HashMap::new(),
            uid: 0,
            sudo_user: None,
            space: Some(Space {
                size: 100 << 30,
                avail: 81 << 30,
            }),
            mounts: Some(vec![mount("/", "btrfs")]),
            now: NOW.parse().unwrap(),
            root: PathBuf::from("/nonexistent-forge-doctor-root"),
            fork_dirs: Vec::new(),
            fork_timeout: Duration::from_secs(5),
        }
    }
}

fn mount(target: &str, fstype: &str) -> Mount {
    Mount {
        target: PathBuf::from(target),
        fstype: fstype.to_owned(),
    }
}

impl Fake {
    fn answer(&mut self, command_line: &str, answer: Answer) -> &mut Self {
        self.commands.insert(command_line.to_owned(), answer);
        self
    }

    fn file(&mut self, path: &str, content: &[u8]) -> &mut Self {
        self.files.insert(PathBuf::from(path), content.to_vec());
        self
    }

    fn dir(&mut self, path: &str) -> &mut Self {
        self.dirs.insert(PathBuf::from(path));
        self
    }

    fn report(&self, config: &Config) -> (String, Totals) {
        self.run(config, false)
    }

    fn run(&self, config: &Config, all: bool) -> (String, Totals) {
        let system = System {
            commands: self,
            files: self,
            host: self,
            clock: self,
            root: self.root.clone(),
            forks: Forks {
                dirs: self.fork_dirs.clone(),
                builtins: vec!["check-docs".to_owned(), "doctor".to_owned()],
                timeout: self.fork_timeout,
            },
        };
        let mut out = Vec::new();
        let options = Options {
            all,
            ..Options::default()
        };
        let totals = doctor::run(config, &system, options, &mut out).unwrap();
        (String::from_utf8(out).unwrap(), totals)
    }

    fn history(&self, config: &Config) -> String {
        let system = System {
            commands: self,
            files: self,
            host: self,
            clock: self,
            root: self.root.clone(),
            forks: Forks {
                dirs: Vec::new(),
                builtins: Vec::new(),
                timeout: self.fork_timeout,
            },
        };
        let mut out = Vec::new();
        doctor::history(config, &system, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }
}

impl Commands for Fake {
    fn run(&self, invocation: &Invocation) -> Answer {
        let user = invocation
            .run_as
            .map(|owner| format!("as {}:{} ", owner.uid, owner.gid))
            .unwrap_or_default();
        let discarded = if invocation.discard_output {
            " >/dev/null"
        } else {
            ""
        };
        self.commands
            .get(&format!("{user}{}{discarded}", invocation.command_line()))
            .cloned()
            .unwrap_or_default()
    }
}

impl Files for Fake {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.dirs.contains(path)
    }

    fn list(&self, dir: &Path) -> Vec<String> {
        self.files
            .keys()
            .chain(&self.dirs)
            .filter(|path| path.parent() == Some(dir))
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect()
    }

    fn canonicalize(&self, path: &Path) -> PathBuf {
        self.links
            .get(path)
            .cloned()
            .unwrap_or_else(|| path.to_path_buf())
    }

    fn owner(&self, path: &Path) -> Option<Owner> {
        self.owners
            .get(path)
            .copied()
            .or(Some(Owner { uid: 0, gid: 0 }))
    }
}

impl Host for Fake {
    fn uid(&self) -> u32 {
        self.uid
    }

    fn sudo_user(&self) -> Option<String> {
        self.sudo_user.clone()
    }

    fn space(&self, _: &Path) -> io::Result<Space> {
        self.space
            .ok_or_else(|| io::Error::from(io::ErrorKind::PermissionDenied))
    }

    fn mounts(&self) -> io::Result<Vec<Mount>> {
        self.mounts
            .clone()
            .ok_or_else(|| io::Error::from(io::ErrorKind::PermissionDenied))
    }
}

impl Clock for Fake {
    fn now(&self) -> Timestamp {
        self.now
    }

    fn zone(&self) -> TimeZone {
        TimeZone::UTC
    }
}

fn line<'a>(output: &'a str, check: &str) -> &'a str {
    output
        .lines()
        .find(|line| line.len() > 7 && line[7..].starts_with(&format!("{check} ")))
        .unwrap_or_else(|| panic!("no {check} line in\n{output}"))
}

fn hint<'a>(output: &'a str, check: &str) -> &'a str {
    let mut lines = output.lines();
    lines.find(|text| text.len() > 7 && text[7..].starts_with(&format!("{check} ")));
    lines.next().unwrap_or_default().trim()
}

fn secure_boot() -> Config {
    Config {
        secure_boot: true,
        ..Config::default()
    }
}

fn efivar(fake: &mut Fake, name: &str, value: u8) {
    fake.file(
        &format!("{EFIVARS}/{name}-{EFI_GLOBAL}"),
        &[7, 0, 0, 0, value],
    );
}

#[test]
fn a_healthy_machine_without_features_reports_every_check() {
    let fake = Fake::default();

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(totals, Totals::default());
    assert_snapshot!(output);
}

#[test]
fn without_root_the_report_says_so_and_skips_the_root_checks() {
    let fake = Fake {
        uid: 1000,
        ..Fake::default()
    };
    let config = Config {
        luks_device: Some(PathBuf::from(LUKS)),
        ..Config::default()
    };

    let (output, _) = fake.report(&config);

    assert!(output.starts_with(
        "Running without root: the checks that need it are skipped. Run sudo forge doctor for all of them.\n\n"
    ));
    assert_eq!(
        line(&output, "LUKS unlock"),
        "[SKIP] LUKS unlock    needs root, run sudo forge doctor"
    );
    assert_eq!(
        line(&output, "Boot"),
        "[SKIP] Boot           needs root, run sudo forge doctor"
    );
}

#[test]
fn secure_boot_is_skipped_without_the_feature() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(
        line(&output, "Secure Boot"),
        "[SKIP] Secure Boot    secure-boot is not imported"
    );
}

#[test]
fn secure_boot_fails_without_uefi() {
    let (output, totals) = Fake::default().report(&secure_boot());

    assert_eq!(
        line(&output, "Secure Boot"),
        "[FAIL] Secure Boot    not booted through UEFI"
    );
    assert_eq!(totals.failures, 1);
}

#[test]
fn secure_boot_fails_in_setup_mode() {
    let mut fake = Fake::default();
    fake.dir("/sys/firmware/efi");
    efivar(&mut fake, "SetupMode", 1);
    efivar(&mut fake, "SecureBoot", 1);

    let (output, _) = fake.report(&secure_boot());

    assert_eq!(
        line(&output, "Secure Boot"),
        "[FAIL] Secure Boot    firmware in setup mode, keys not enrolled"
    );
    assert_eq!(
        hint(&output, "Secure Boot"),
        "reboot: systemd-boot enrolls the keys from /boot/loader/keys/auto"
    );
}

#[test]
fn secure_boot_is_ok_when_enforced() {
    let mut fake = Fake::default();
    fake.dir("/sys/firmware/efi");
    efivar(&mut fake, "SetupMode", 0);
    efivar(&mut fake, "SecureBoot", 1);

    let (output, _) = fake.report(&secure_boot());

    assert_eq!(
        line(&output, "Secure Boot"),
        "[  OK] Secure Boot    enforced, keys enrolled"
    );
}

#[test]
fn secure_boot_fails_when_disabled() {
    let mut fake = Fake::default();
    fake.dir("/sys/firmware/efi");
    efivar(&mut fake, "SecureBoot", 0);

    let (output, _) = fake.report(&secure_boot());

    assert_eq!(
        line(&output, "Secure Boot"),
        "[FAIL] Secure Boot    disabled in the firmware"
    );
}

fn luks() -> Config {
    Config {
        luks_device: Some(PathBuf::from(LUKS)),
        ..Config::default()
    }
}

fn keyslots(types: &[&str]) -> Answer {
    let rows: Vec<String> = types
        .iter()
        .enumerate()
        .map(|(slot, kind)| format!("{slot:>4} {kind}"))
        .collect();
    Answer::ok(&format!(
        "SLOT TYPE
{}
",
        rows.join(
            "
"
        )
    ))
}

#[test]
fn luks_is_skipped_without_disk_and_secure_boot() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(
        line(&output, "LUKS unlock"),
        "[SKIP] LUKS unlock    needs disk and secure-boot"
    );
}

#[test]
fn luks_fails_when_the_keyslots_cannot_be_read() {
    let mut fake = Fake::default();
    fake.answer(
        &format!("systemd-cryptenroll {LUKS}"),
        Answer::failed("Failed to open device: No such device\n"),
    );

    let (output, _) = fake.report(&luks());

    assert_eq!(
        line(&output, "LUKS unlock"),
        format!("[FAIL] LUKS unlock    cannot read the keyslots of {LUKS}")
    );
    assert_eq!(
        hint(&output, "LUKS unlock"),
        "Failed to open device: No such device"
    );
}

#[test]
fn luks_fails_without_a_passphrase() {
    let mut fake = Fake::default();
    fake.answer(&format!("systemd-cryptenroll {LUKS}"), keyslots(&["tpm2"]));

    let (output, _) = fake.report(&luks());

    assert_eq!(
        line(&output, "LUKS unlock"),
        "[FAIL] LUKS unlock    no passphrase keyslot, nothing to fall back on if the TPM2 refuses"
    );
    assert_eq!(
        hint(&output, "LUKS unlock"),
        format!("add one now: sudo systemd-cryptenroll --password {LUKS}")
    );
}

#[test]
fn luks_warns_without_a_tpm2_token() {
    let mut fake = Fake::default();
    fake.answer(
        &format!("systemd-cryptenroll {LUKS}"),
        keyslots(&["recovery"]),
    );

    let (output, totals) = fake.report(&luks());

    assert_eq!(
        line(&output, "LUKS unlock"),
        "[WARN] LUKS unlock    no TPM2 token, the passphrase is asked at every boot"
    );
    assert_eq!(totals.warnings, 1);
}

#[test]
fn luks_is_ok_with_a_token_and_a_passphrase() {
    let mut fake = Fake::default();
    fake.answer(
        &format!("systemd-cryptenroll {LUKS}"),
        keyslots(&["password", "tpm2"]),
    );

    let (output, _) = fake.report(&luks());

    assert_eq!(
        line(&output, "LUKS unlock"),
        "[  OK] LUKS unlock    TPM2 token, passphrase kept as a fallback"
    );
}

fn snapshots() -> Config {
    Config {
        btrbk_calendar: Some("hourly".to_owned()),
        snapshot_dir: PathBuf::from("/.snapshots"),
        snapshot_subvolumes: vec!["root".to_owned(), "home".to_owned()],
        ..Config::default()
    }
}

fn btrbk(fake: &mut Fake, result: &str, started_ago: i64) {
    let started = fake.now.as_second() - started_ago;
    fake.answer(
        "systemctl show -p Result --value btrbk-snapshots.service",
        Answer::ok(&format!("{result}\n")),
    )
    .answer(
        "systemd-analyze calendar --iterations=2 hourly",
        Answer::ok(
            "  Original form: hourly\n\
             Normalized form: *-*-* *:00:00\n    \
             Next elapse: Fri 2026-10-23 13:00:00 UTC\n       \
             From now: 59min left\n   \
             Iteration #2: Fri 2026-10-23 14:00:00 UTC\n",
        ),
    )
    .answer(
        "systemctl show --timestamp=unix -p ExecMainStartTimestamp --value btrbk-snapshots.service",
        Answer::ok(&if started_ago < 0 {
            "\n".to_owned()
        } else {
            format!("@{started}\n")
        }),
    );
}

#[test]
fn snapshots_are_skipped_without_btrbk() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(
        line(&output, "Snapshots"),
        "[SKIP] Snapshots      no btrbk snapshots on this host"
    );
}

#[test]
fn snapshots_fail_after_a_failed_run() {
    let mut fake = Fake::default();
    btrbk(&mut fake, "exit-code", 600);

    let (output, _) = fake.report(&snapshots());

    assert_eq!(
        line(&output, "Snapshots"),
        "[FAIL] Snapshots      the last btrbk run failed"
    );
}

#[test]
fn snapshots_warn_when_btrbk_never_ran() {
    let mut fake = Fake::default();
    btrbk(&mut fake, "success", -1);

    let (output, _) = fake.report(&snapshots());

    assert_eq!(
        line(&output, "Snapshots"),
        "[WARN] Snapshots      btrbk never ran"
    );
}

#[test]
fn snapshots_warn_past_twice_the_interval() {
    let mut fake = Fake::default();
    btrbk(&mut fake, "success", 3 * 3600);

    let (output, _) = fake.report(&snapshots());

    assert_eq!(
        line(&output, "Snapshots"),
        "[WARN] Snapshots      last btrbk run 3 h ago, scheduled every 1 h"
    );
}

#[test]
fn a_trigger_newer_than_the_start_counts_as_the_last_run() {
    let mut fake = Fake::default();
    btrbk(&mut fake, "success", 3 * 3600);
    let triggered = fake.now.as_second() - 600;
    fake.answer(
        "systemctl show --timestamp=unix -p LastTriggerUSec --value btrbk-snapshots.timer",
        Answer::ok(&format!("@{triggered}\n")),
    )
    .dir("/.snapshots/root.20261023T1150")
    .dir("/.snapshots/home.20261023T1150");

    let (output, _) = fake.report(&snapshots());

    assert_eq!(
        line(&output, "Snapshots"),
        "[  OK] Snapshots      last btrbk run 10 min ago, scheduled every 1 h"
    );
}

#[test]
fn snapshots_warn_about_a_subvolume_without_snapshot() {
    let mut fake = Fake::default();
    btrbk(&mut fake, "success", 600);
    fake.dir("/.snapshots/root.20261023T1150");

    let (output, _) = fake.report(&snapshots());

    assert_eq!(
        line(&output, "Snapshots"),
        "[WARN] Snapshots      no snapshot of home in /.snapshots"
    );
}

#[test]
fn snapshots_are_ok_with_a_recent_run_and_every_subvolume() {
    let mut fake = Fake::default();
    btrbk(&mut fake, "success", 600);
    fake.dir("/.snapshots/root.20261023T1150")
        .dir("/.snapshots/home.20261023T1150");

    let (output, _) = fake.report(&snapshots());

    assert_eq!(
        line(&output, "Snapshots"),
        "[  OK] Snapshots      last btrbk run 10 min ago, scheduled every 1 h"
    );
}

#[test]
fn an_unreadable_calendar_counts_as_hourly() {
    let mut fake = Fake::default();
    btrbk(&mut fake, "success", 600);
    fake.answer(
        "systemd-analyze calendar --iterations=2 hourly",
        Answer::default(),
    )
    .dir("/.snapshots/root.1")
    .dir("/.snapshots/home.1");

    let (output, _) = fake.report(&snapshots());

    assert!(line(&output, "Snapshots").ends_with("scheduled every 1 h"));
}

#[test]
fn disk_space_is_ok_above_ten_percent() {
    let fake = Fake {
        space: Some(Space {
            size: 500 << 30,
            avail: 380 << 30,
        }),
        ..Fake::default()
    };

    let (output, _) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "Disk space"),
        "[  OK] Disk space     76 % free on / (btrfs, 380G left)"
    );
}

#[test]
fn disk_space_warns_under_ten_percent() {
    let fake = Fake {
        space: Some(Space {
            size: 100 << 30,
            avail: 1536 << 20,
        }),
        mounts: Some(vec![mount("/", "tmpfs"), mount("/", "ext4")]),
        ..Fake::default()
    };

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "Disk space"),
        "[WARN] Disk space     1 % free on / (ext4, 1.5G left)"
    );
    assert_eq!(
        hint(&output, "Disk space"),
        "nh clean all --keep 3, and drop old snapshots in /.snapshots"
    );
    assert_eq!(totals.warnings, 1);
}

const FAILED: &str = "list-units --failed --plain --no-legend --no-pager";

#[test]
fn no_failed_unit_is_ok() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(line(&output, "Failed units"), "[  OK] Failed units   none");
}

#[test]
fn failed_system_units_fail() {
    let mut fake = Fake::default();
    fake.answer(
        &format!("systemctl {FAILED}"),
        Answer::ok("doctor-broken.service loaded failed failed /run/current-system/sw/bin/false\nother.service loaded failed failed x\n"),
    );

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "Failed units"),
        "[FAIL] Failed units   system: doctor-broken.service other.service"
    );
    assert_eq!(totals.failures, 1);
}

#[test]
fn failed_user_units_of_the_sudo_user_fail() {
    let mut fake = Fake {
        sudo_user: Some("alice".to_owned()),
        ..Fake::default()
    };
    fake.answer(
        &format!("systemctl --user --machine=alice@ {FAILED}"),
        Answer::ok("swaync.service loaded failed failed x\n"),
    );

    let (output, _) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "Failed units"),
        "[FAIL] Failed units   user: swaync.service"
    );
}

#[test]
fn failed_system_and_user_units_are_both_listed() {
    let mut fake = Fake {
        uid: 1000,
        ..Fake::default()
    };
    fake.answer(
        &format!("systemctl {FAILED}"),
        Answer::ok("a.service loaded failed failed x\n"),
    )
    .answer(
        &format!("systemctl --user {FAILED}"),
        Answer::ok("b.service loaded failed failed x\n"),
    );

    let (output, _) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "Failed units"),
        "[FAIL] Failed units   system: a.service; user: b.service"
    );
    assert_eq!(
        hint(&output, "Failed units"),
        "systemctl status <unit>, journalctl -b -u <unit> (--user for user units)"
    );
}

fn updates() -> Config {
    Config {
        update_flake: Some(PathBuf::from(FLAKE)),
        ..Config::default()
    }
}

fn checkout(fake: &mut Fake, fetched: bool, counts: &str, status: &str) {
    fake.dir(&format!("{FLAKE}/.git"));
    fake.owners.insert(
        PathBuf::from(FLAKE),
        Owner {
            uid: 1000,
            gid: 100,
        },
    );
    let fetch = if fetched {
        Answer::ok("")
    } else {
        Answer::failed("fatal")
    };
    fake.answer(
        &format!("{SETPRIV} git -C {FLAKE} fetch --quiet >/dev/null"),
        fetch,
    )
    .answer(
        &format!("{SETPRIV} git -C {FLAKE} rev-list --left-right --count @{{upstream}}...HEAD"),
        if counts.is_empty() {
            Answer::failed("fatal: no upstream")
        } else {
            Answer::ok(counts)
        },
    )
    .answer(
        &format!("{SETPRIV} git -C {FLAKE} status --porcelain"),
        Answer::ok(status),
    );
}

#[test]
fn updates_are_skipped_without_a_checkout() {
    let (output, _) = Fake::default().report(&updates());

    assert_eq!(
        line(&output, "Updates"),
        "[SKIP] Updates        the host's flake is not a git checkout on this machine"
    );
}

#[test]
fn updates_warn_when_the_checkout_has_no_upstream() {
    let mut fake = Fake::default();
    checkout(&mut fake, true, "", "");

    let (output, _) = fake.report(&updates());

    assert_eq!(
        line(&output, "Updates"),
        format!("[WARN] Updates        cannot compare {FLAKE} with its upstream branch")
    );
    assert_eq!(
        hint(&output, "Updates"),
        format!("git -C {FLAKE} branch --set-upstream-to origin/main")
    );
}

#[test]
fn updates_warn_behind_its_upstream_and_git_runs_as_the_owner() {
    let mut fake = Fake::default();
    checkout(&mut fake, true, "1\t0\n", "");

    let (output, _) = fake.report(&updates());

    assert_eq!(
        line(&output, "Updates"),
        format!("[WARN] Updates        {FLAKE}: behind by 1")
    );
    assert_eq!(hint(&output, "Updates"), "run update");
}

#[test]
fn updates_warn_ahead_and_dirty_with_a_failed_fetch() {
    let mut fake = Fake::default();
    checkout(&mut fake, false, "0\t2\n", " M dirty\n");

    let (output, _) = fake.report(&updates());

    assert_eq!(
        line(&output, "Updates"),
        format!(
            "[WARN] Updates        {FLAKE}: ahead by 2, uncommitted changes, fetch failed, counts from the last fetch"
        )
    );
    assert_eq!(
        hint(&output, "Updates"),
        "commit and push, so other machines and CI get it"
    );
}

#[test]
fn updates_are_ok_in_step_with_the_upstream() {
    let mut fake = Fake::default();
    checkout(&mut fake, true, "0\t0\n", "");

    let (output, _) = fake.report(&updates());

    assert_eq!(
        line(&output, "Updates"),
        format!("[  OK] Updates        {FLAKE} is up to date with its upstream")
    );
}

#[test]
fn a_checkout_owned_by_the_caller_runs_git_directly() {
    let mut fake = Fake {
        uid: 1000,
        ..Fake::default()
    };
    fake.dir(&format!("{FLAKE}/.git"));
    fake.owners.insert(
        PathBuf::from(FLAKE),
        Owner {
            uid: 1000,
            gid: 100,
        },
    );
    fake.answer(
        &format!("git -C {FLAKE} fetch --quiet >/dev/null"),
        Answer::ok(""),
    )
    .answer(
        &format!("git -C {FLAKE} rev-list --left-right --count @{{upstream}}...HEAD"),
        Answer::ok("0\t0\n"),
    );

    let (output, _) = fake.report(&updates());

    assert_eq!(
        line(&output, "Updates"),
        format!("[  OK] Updates        {FLAKE} is up to date with its upstream")
    );
}

fn upgrades() -> Config {
    Config {
        auto_upgrade_state: Some(PathBuf::from(STATE)),
        deploy_signers: 1,
        ..Config::default()
    }
}

fn state(fake: &mut Fake, json: &str) {
    fake.file(&format!("{STATE}/state.json"), json.as_bytes());
}

fn history(fake: &mut Fake, rows: &[(&str, &str)]) {
    let text: Vec<String> = rows
        .iter()
        .map(|(time, outcome)| format!("{time}	{outcome}	a	b	https://example.org/fork.git"))
        .collect();
    fake.file(
        &format!("{STATE}/history.tsv"),
        text.join(
            "
",
        )
        .as_bytes(),
    );
}

#[test]
fn upgrades_are_skipped_without_auto_upgrade() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(
        line(&output, "Upgrades"),
        "[SKIP] Upgrades       auto-upgrade is off"
    );
}

#[test]
fn upgrades_are_skipped_before_the_first_run() {
    let (output, _) = Fake::default().report(&upgrades());

    assert_eq!(
        line(&output, "Upgrades"),
        "[SKIP] Upgrades       never ran yet"
    );
}

#[test]
fn an_unreadable_state_warns_and_the_other_checks_still_run() {
    for broken in [
        r#"{"time":"x""#,
        r#"{"time":"not-a-date","outcome":"upgraded"}"#,
        r#"{"outcome":"upgraded"}"#,
    ] {
        let mut fake = Fake::default();
        state(&mut fake, broken);

        let (output, _) = fake.report(&upgrades());

        assert_eq!(
            line(&output, "Upgrades"),
            format!("[WARN] Upgrades       cannot read {STATE}/state.json"),
            "{broken}"
        );
        assert!(output.contains("\nfailed: "), "{broken}");
    }
}

#[test]
fn a_refused_upgrade_warns_with_its_reason() {
    let mut fake = Fake::default();
    state(
        &mut fake,
        r#"{"time":"2026-10-23T11:00:00+00:00","outcome":"refused","message":"this machine runs uncommitted changes"}"#,
    );

    let (output, _) = fake.report(&upgrades());

    assert_eq!(
        line(&output, "Upgrades"),
        "[WARN] Upgrades       refused on 2026-10-23 11:00"
    );
    assert_eq!(
        hint(&output, "Upgrades"),
        "this machine runs uncommitted changes"
    );
}

#[test]
fn a_rollback_warns_with_its_reason() {
    let mut fake = Fake::default();
    state(
        &mut fake,
        r#"{"time":"2026-10-23T13:00:00+02:00","outcome":"rolled-back","message":"b did not start"}"#,
    );

    let (output, _) = fake.report(&upgrades());

    assert_eq!(
        line(&output, "Upgrades"),
        "[WARN] Upgrades       rolled-back on 2026-10-23 11:00"
    );
    assert_eq!(hint(&output, "Upgrades"), "b did not start");
}

#[test]
fn a_success_older_than_two_weeks_warns() {
    let mut fake = Fake::default();
    state(
        &mut fake,
        r#"{"time":"2026-10-03T12:00:00+00:00","outcome":"upgraded","message":"applies at next boot"}"#,
    );

    let (output, _) = fake.report(&upgrades());

    assert_eq!(
        line(&output, "Upgrades"),
        "[WARN] Upgrades       last run on 2026-10-03 12:00, 20 days ago"
    );
}

#[test]
fn an_upgrade_says_what_changed() {
    let mut fake = Fake::default();
    state(
        &mut fake,
        r#"{"time":"2026-10-22T04:00:00+00:00","outcome":"upgraded","message":"applies at next boot","changes":"Closure size: 10 -> 12 (3 paths added)"}"#,
    );

    let (output, _) = fake.report(&upgrades());

    assert_eq!(
        line(&output, "Upgrades"),
        "[  OK] Upgrades       upgraded 2026-10-22 04:00: Closure size: 10 -> 12 (3 paths added)"
    );
}

#[test]
fn an_upgrade_without_changes_shows_its_message() {
    let mut fake = Fake::default();
    state(
        &mut fake,
        r#"{"time":"2026-10-22T04:00:00+00:00","outcome":"upgraded","message":"applies at next boot","changes":null}"#,
    );

    let (output, _) = fake.report(&upgrades());

    assert_eq!(
        line(&output, "Upgrades"),
        "[  OK] Upgrades       upgraded 2026-10-22 04:00: applies at next boot"
    );
}

fn up_to_date(fake: &mut Fake) {
    state(
        fake,
        r#"{"time":"2026-10-23T04:00:00+00:00","outcome":"up-to-date","message":"deploy/machine is the running commit"}"#,
    );
}

#[test]
fn up_to_date_is_dated_from_the_last_upgrade() {
    let mut fake = Fake::default();
    up_to_date(&mut fake);
    history(
        &mut fake,
        &[
            ("2026-10-01T04:00:00+00:00", "waiting"),
            ("2026-10-02T04:00:00+00:00", "upgraded"),
            ("2026-10-03T04:00:00+00:00", "up-to-date"),
        ],
    );

    let (output, _) = fake.report(&upgrades());

    assert_eq!(
        line(&output, "Upgrades"),
        "[  OK] Upgrades       up to date since 2026-10-02 04:00"
    );
}

#[test]
fn an_unreachable_night_does_not_move_the_date() {
    let mut fake = Fake::default();
    up_to_date(&mut fake);
    history(
        &mut fake,
        &[
            ("2026-10-01T04:00:00+00:00", "upgraded"),
            ("2026-10-02T04:00:00+00:00", "unreachable"),
            ("2026-10-17T04:00:00+00:00", "up-to-date"),
        ],
    );

    let (output, _) = fake.report(&upgrades());

    assert!(line(&output, "Upgrades").ends_with("up to date since 2026-10-01 04:00"));
}

#[test]
fn a_rollback_restarts_the_date_at_the_next_up_to_date_run() {
    let mut fake = Fake::default();
    up_to_date(&mut fake);
    history(
        &mut fake,
        &[
            ("2026-10-01T04:00:00+00:00", "upgraded"),
            ("2026-10-02T04:00:00+00:00", "rolled-back"),
            ("2026-10-05T04:00:00+00:00", "up-to-date"),
            ("2026-10-06T04:00:00+00:00", "up-to-date"),
        ],
    );

    let (output, _) = fake.report(&upgrades());

    assert!(line(&output, "Upgrades").ends_with("up to date since 2026-10-05 04:00"));
}

#[test]
fn without_history_up_to_date_is_dated_from_the_last_run() {
    let mut fake = Fake::default();
    up_to_date(&mut fake);

    let (output, _) = fake.report(&upgrades());

    assert!(line(&output, "Upgrades").ends_with("up to date since 2026-10-23 04:00"));
}

#[test]
fn signers_are_skipped_without_auto_upgrade() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(
        line(&output, "Signed deploys"),
        "[SKIP] Signed deploys auto-upgrade is off"
    );
}

#[test]
fn no_signer_warns_that_any_deploy_commit_applies() {
    let config = Config {
        deploy_signers: 0,
        ..upgrades()
    };

    let (output, _) = Fake::default().report(&config);

    assert_eq!(
        line(&output, "Signed deploys"),
        "[WARN] Signed deploys auto-upgrade applies any deploy commit its sources serve, signed or not"
    );
    assert!(hint(&output, "Signed deploys").contains("update.signers"));
}

#[test]
fn signers_are_counted_in_keys() {
    let (one, _) = Fake::default().report(&upgrades());
    let (two, _) = Fake::default().report(&Config {
        deploy_signers: 2,
        ..upgrades()
    });

    assert_eq!(
        line(&one, "Signed deploys"),
        "[  OK] Signed deploys auto-upgrade applies only deploy commits signed by 1 key of update.signers"
    );
    assert!(line(&two, "Signed deploys").contains("signed by 2 keys of"));
}

const BOOTCTL: &str = "bootctl list --json=short --no-pager";

fn entries(fake: &mut Fake, newest: &str) {
    fake.answer(
        BOOTCTL,
        Answer::ok(&format!(
            r#"[{{"version":"Generation 1 NixOS","path":"/boot/loader/entries/nixos-generation-1.conf"}},{{"version":"Generation 2 NixOS","path":"/boot/loader/entries/{newest}"}},{{"version":null,"path":"/boot/EFI/memtest.efi"}}]"#
        )),
    );
}

#[test]
fn boot_is_skipped_without_systemd_boot_entries() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(
        line(&output, "Boot"),
        "[SKIP] Boot           no systemd-boot entries on this machine"
    );
}

#[test]
fn a_generation_that_spent_its_tries_warns_of_the_rollback() {
    let mut fake = Fake::default();
    entries(&mut fake, "nixos-generation-2+0-3.conf");
    fake.links.insert(
        PathBuf::from("/nix/var/nix/profiles/system-2-link"),
        PathBuf::from("/nix/store/two"),
    );
    fake.links.insert(
        PathBuf::from("/run/current-system"),
        PathBuf::from("/nix/store/one"),
    );

    let (without, _) = fake.report(&Config::default());
    let (with, _) = fake.report(&upgrades());

    assert_eq!(
        line(&without, "Boot"),
        "[WARN] Boot           the last update did not start, this machine went back to the previous version"
    );
    assert_eq!(hint(&without, "Boot"), "journalctl -b -1 shows why");
    assert_eq!(
        hint(&with, "Boot"),
        "journalctl -b -1 shows why, forge doctor --history lists the upgrades"
    );
}

#[test]
fn a_spent_generation_that_runs_is_ok() {
    let mut fake = Fake::default();
    entries(&mut fake, "nixos-generation-2+0-3.conf");
    for link in ["/nix/var/nix/profiles/system-2-link", "/run/current-system"] {
        fake.links
            .insert(PathBuf::from(link), PathBuf::from("/nix/store/two"));
    }

    let (output, _) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "Boot"),
        "[  OK] Boot           the newest generation started, or has tries left"
    );
}

#[test]
fn a_generation_with_tries_left_is_ok() {
    let mut fake = Fake::default();
    entries(&mut fake, "nixos-generation-2+1-2.conf");

    let (output, _) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "Boot"),
        "[  OK] Boot           the newest generation started, or has tries left"
    );
}

#[test]
fn history_says_when_auto_upgrade_is_off_or_never_ran() {
    assert_eq!(
        Fake::default().history(&Config::default()),
        "auto-upgrade is off on this host\n"
    );
    assert_eq!(
        Fake::default().history(&upgrades()),
        "auto-upgrade never ran yet\n"
    );
}

#[test]
fn history_lists_the_runs_without_the_checks() {
    let mut fake = Fake::default();
    fake.file(
        &format!("{STATE}/history.tsv"),
        b"2026-01-01T04:00:00+00:00\twaiting\t\t\t\n\
          2026-01-02T04:00:00+00:00\tupgraded\taaaaaaaaaaaaaaaaaaaa\tbbbbbbbbbbbbbbbbbbbb\thttps://example.org/fork.git\n",
    );

    assert_snapshot!(fake.history(&upgrades()));
}

struct Disk {
    dir: TempDir,
    since: Timestamp,
}

impl Disk {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let since = Timestamp::now() - jiff::SignedDuration::from_secs(60);
        Self { dir, since }
    }

    fn write(&self, relative: &str) {
        let path = self.dir.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "state").unwrap();
    }

    fn link(&self, relative: &str, target: &str) {
        let path = self.dir.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, path).unwrap();
    }

    fn fake(&self) -> Fake {
        let mut fake = Fake {
            root: self.dir.path().to_path_buf(),
            mounts: Some(vec![mount("/", "btrfs"), mount("/persist", "btrfs")]),
            ..Fake::default()
        };
        let micros = self.since.as_microsecond();
        let wall = self
            .since
            .strftime("%a %Y-%m-%d %H:%M:%S.%6f UTC")
            .to_string();
        fake.answer(
            BOOT_SETUP,
            Answer::ok(&format!(
                "Id=sysinit.target\nActiveEnterTimestamp={wall}\nActiveEnterTimestampMonotonic={}\n",
                micros % 1_000_000_000 + 1
            )),
        );
        fake
    }
}

fn wiped() -> Config {
    Config {
        wiped_root: true,
        ephemeral_paths: vec![PathBuf::from("/tmp"), PathBuf::from("/var/lib/lastlog")],
        ..Config::default()
    }
}

#[test]
fn impermanence_is_skipped_when_root_is_kept() {
    let (output, _) = Fake::default().report(&Config::default());

    assert_eq!(
        line(&output, "Impermanence"),
        "[SKIP] Impermanence   / is not wiped at boot on this host"
    );
}

#[test]
fn nothing_written_since_boot_is_ok() {
    let disk = Disk::new();
    disk.write("tmp/scratch");
    disk.write("var/lib/lastlog");
    disk.write("persist/kept");
    disk.write("etc/passwd");
    disk.write("etc/hosts");
    disk.link("etc/static-link", "/etc/static/zshrc");
    disk.link("run-link", "/nix/store/abc-system");
    let mut fake = disk.fake();
    fake.file("/etc/.clean", b"hosts\n");

    let (output, _) = fake.report(&wiped());

    assert_eq!(
        line(&output, "Impermanence"),
        "[  OK] Impermanence   nothing written to / since boot is lost at the next boot"
    );
}

#[test]
fn an_unreadable_mount_table_is_reported_instead_of_walking() {
    let disk = Disk::new();
    disk.write("proc/self/lost");
    let mut fake = disk.fake();
    fake.mounts = None;

    let (output, totals) = fake.report(&wiped());

    assert_eq!(
        line(&output, "Impermanence"),
        "[WARN] Impermanence   cannot read the mount table: permission denied"
    );
    assert!(!output.contains("proc"), "{output}");
    assert_eq!(totals.warnings, 1);
}

#[test]
fn files_written_since_boot_are_summarised_by_directory() {
    let disk = Disk::new();
    disk.write("doctor-lost");
    disk.write("srv/doctor/state");
    disk.write("srv/doctor/other");
    disk.write("var/lib/doctor-late");
    disk.write("a/b/c/d/deep");
    disk.link("var/lib/link", "/var/lib/doctor-late");
    let fake = disk.fake();

    let (output, totals) = fake.report(&wiped());

    assert_eq!(totals.warnings, 1);
    assert_snapshot!(output);
}

#[test]
fn all_lists_every_lost_file() {
    let disk = Disk::new();
    disk.write("doctor-lost");
    disk.write("srv/doctor/state");
    let fake = Fake {
        uid: 1000,
        ..disk.fake()
    };

    let (output, _) = fake.run(&wiped(), true);

    assert_snapshot!(output);
}

#[test]
fn files_older_than_the_boot_setup_are_not_lost() {
    let disk = Disk::new();
    disk.write("old");
    let mut fake = disk.fake();
    let later = Timestamp::now() + jiff::SignedDuration::from_secs(60);
    let wall = later.strftime("%a %Y-%m-%d %H:%M:%S.%6f UTC").to_string();
    fake.answer(
        BOOT_SETUP,
        Answer::ok(&format!(
            "Id=sysinit.target\nActiveEnterTimestamp={wall}\nActiveEnterTimestampMonotonic=5000000\n"
        )),
    );

    let (output, _) = fake.report(&wiped());

    assert!(
        line(&output, "Impermanence").starts_with("[  OK]"),
        "{output}"
    );
}

fn fork_command(dir: &Path, name: &str, describe: &str, doctor: &str) {
    let path = dir.join(format!("forge-{name}"));
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --forge-describe ]; then\n  echo '{describe}'\n  exit 0\nfi\n{doctor}\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn describe(name: &str, doctor: bool) -> String {
    let field = if doctor { r#","doctor":true"# } else { "" };
    format!(r#"{{"contract":1,"name":"{name}","summary":"s"{field}}}"#)
}

#[test]
fn fork_checks_are_reported_and_counted() {
    let dir = tempfile::tempdir().unwrap();
    fork_command(
        dir.path(),
        "backup-age",
        &describe("backup-age", true),
        r#"echo '{"status":"WARN","check":"Backup age","detail":"last backup 3 days ago, root='"$FORGE_DOCTOR_ROOT"' contract='"$FORGE_CONTRACT"'","hint":"systemctl start backup"}'
echo '{"status":"OK","check":"Backup target","detail":"reachable"}'"#,
    );
    fork_command(
        dir.path(),
        "hello",
        &describe("hello", false),
        r#"echo '{"status":"FAIL","check":"Hello","detail":"must not run"}'"#,
    );
    let fake = Fake {
        fork_dirs: vec![dir.path().to_path_buf()],
        ..Fake::default()
    };

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(
        totals,
        Totals {
            failures: 0,
            warnings: 1
        }
    );
    assert_snapshot!(output);
}

#[test]
fn an_invalid_line_fails_after_the_valid_ones() {
    let dir = tempfile::tempdir().unwrap();
    fork_command(
        dir.path(),
        "broken",
        &describe("broken", true),
        r#"echo '{"status":"OK","check":"First","detail":"fine"}'
echo 'not json'
echo '{"status":"OK","check":"Never","detail":"after the invalid line"}'"#,
    );
    fork_command(
        dir.path(),
        "long",
        &describe("long", true),
        r#"echo '{"status":"OK","check":"A name far too long","detail":"x"}'"#,
    );
    let fake = Fake {
        fork_dirs: vec![dir.path().to_path_buf()],
        ..Fake::default()
    };

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(line(&output, "First"), "[  OK] First          fine");
    assert!(!output.contains("Never"));
    assert!(
        line(&output, "forge-broken")
            .starts_with("[FAIL] forge-broken   printed an invalid line: expected ident")
    );
    assert_eq!(
        line(&output, "forge-long"),
        "[FAIL] forge-long     printed an invalid line: check `A name far too long` must have 1 to 14 characters"
    );
    assert_eq!(totals.failures, 2);
}

#[test]
fn a_fork_check_that_exits_non_zero_or_prints_nothing_fails() {
    let dir = tempfile::tempdir().unwrap();
    fork_command(dir.path(), "crash", &describe("crash", true), "exit 1");
    fork_command(dir.path(), "silent", &describe("silent", true), "exit 0");
    let fake = Fake {
        fork_dirs: vec![dir.path().to_path_buf()],
        ..Fake::default()
    };

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "forge-crash"),
        "[FAIL] forge-crash    `forge-crash --forge-doctor` failed (exit status: 1)"
    );
    assert_eq!(
        line(&output, "forge-silent"),
        "[FAIL] forge-silent   `forge-silent --forge-doctor` printed no check"
    );
    assert_eq!(totals.failures, 2);
}

#[test]
fn a_fork_check_that_never_stops_printing_fails() {
    let dir = tempfile::tempdir().unwrap();
    fork_command(
        dir.path(),
        "chatty",
        &describe("chatty", true),
        r#"yes '{"status":"OK","check":"Chatty","detail":"again"}'"#,
    );
    let fake = Fake {
        fork_dirs: vec![dir.path().to_path_buf()],
        ..Fake::default()
    };

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "forge-chatty"),
        "[FAIL] forge-chatty   `forge-chatty --forge-doctor` printed more than 1048576 bytes, its output was cut"
    );
    assert!(!output.contains("again"));
    assert_eq!(totals.failures, 1);
}

#[test]
fn a_fork_check_that_hangs_is_stopped() {
    let dir = tempfile::tempdir().unwrap();
    fork_command(dir.path(), "sleepy", &describe("sleepy", true), "sleep 30");
    let fake = Fake {
        fork_dirs: vec![dir.path().to_path_buf()],
        fork_timeout: Duration::from_millis(300),
        ..Fake::default()
    };
    let started = Instant::now();

    let (output, totals) = fake.report(&Config::default());

    assert_eq!(
        line(&output, "forge-sleepy"),
        "[FAIL] forge-sleepy   `forge-sleepy --forge-doctor` did not answer within 0.3s and was stopped"
    );
    assert_eq!(totals.failures, 1);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn the_report_ends_with_the_totals() {
    let mut fake = Fake::default();
    fake.answer(
        &format!("systemctl {FAILED}"),
        Answer::ok("a.service loaded failed failed x\n"),
    );

    let (output, totals) = fake.report(&upgrades());

    assert!(output.ends_with("\n\nfailed: 1, warnings: 0\n"), "{output}");
    assert_eq!(totals.failures, 1);
}

#[test]
fn a_missing_config_says_this_is_no_nixos_forge_host() {
    let error = Config::load(Path::new("/nonexistent/doctor.json")).unwrap_err();

    assert_eq!(
        error.to_string(),
        "no /nonexistent/doctor.json, this machine is not a nixos-forge host"
    );
}

#[test]
fn an_invalid_config_names_the_file_and_the_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doctor.json");
    fs::write(&path, r#"{"secureBoot":true,"unknown":1}"#).unwrap();

    let error = Config::load(&path).unwrap_err();

    assert!(
        error.to_string().starts_with(&format!(
            "invalid {}: unknown field `unknown`",
            path.display()
        )),
        "{error}"
    );
}

#[test]
fn the_config_the_doctor_module_writes_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doctor.json");
    fs::write(
        &path,
        r#"{"autoUpgradeState":"/var/lib/auto-upgrade","btrbkCalendar":"hourly","deploySigners":1,"ephemeralPaths":["/tmp"],"luksDevice":null,"secureBoot":false,"snapshotDir":"/.snapshots","snapshotSubvolumes":["home","root"],"updateFlake":null,"wipedRoot":true}"#,
    )
    .unwrap();

    let config = Config::load(&path).unwrap();

    assert_eq!(config.btrbk_calendar.as_deref(), Some("hourly"));
    assert_eq!(config.deploy_signers, 1);
    assert!(config.update_flake.is_none());
}
