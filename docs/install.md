# Install on a machine

From a host file to a real machine that boots, unlocks itself with the TPM2 and rolls back a bad upgrade on its own. The starter is developed and tested in VMs; this page is the path for your fork when you install it on real hardware.

> [!WARNING]
> A host that lists `disk` in `features` gets the encrypted layout below, and **the install erases the whole disk it is given**. Read every step before running one.

The registry never names that disk: it is chosen at install time. Three ways to install:

| Path | When | Picks the disk with |
|---|---|---|
| [Installer ISO](#installer-iso) | the main path: a USB stick, nothing to type but answers | a list on screen |
| [nixos-anywhere](#remote-install-with-nixos-anywhere) | a machine reachable over SSH | `disk.device` in the host |
| [disko-install](#by-hand-with-disko-install) | by hand, from any NixOS ISO with your fork cloned | `--disk main <device>` |

`disk.device` defaults to `/dev/null/no-disk-chosen`, a path that can never exist: a command run without a device fails instead of erasing a disk. The installed system mounts its partitions by label (`/dev/disk/by-partlabel/disk-main-ESP`, `disk-main-luks`), so the device only matters during the install and the same build fits any disk.

## What the disk looks like

| Partition | Content |
|---|---|
| ESP, 1 GiB | vfat on `/boot` |
| rest of the disk | LUKS2 `cryptroot`, then btrfs |

btrfs subvolumes, mounted with `compress=zstd` and `noatime` (`@swap` with `noatime` only):

| Subvolume | Mount | Kept across reboots |
|---|---|---|
| `@root` | `/` | no: restored from `@root-blank` at every boot |
| `@nix` | `/nix` | yes |
| `@persist` | `/persist` | yes |
| `@home` | `/home` | yes, unless the host imports `wipe-home`: then emptied at every boot ([wipe-home.md](wipe-home.md)) |
| `@snapshots` | `/.snapshots` | yes: hourly snapshots of `@home` and `@persist` ([services.md](services.md#snapshots-btrbk)) |
| `@swap` | `/swap` | yes: only with `hibernate`, a swap file for the hibernation image, never snapshotted ([hibernate.md](hibernate.md)) |

Every boot, the initrd deletes `@root` and snapshots it again from the empty `@root-blank`. Anything outside `/nix`, `/home`, `/persist` and `/.snapshots` is gone unless it is declared in `environment.persistence."/persist"`:

- **Kept by the starter**: logs, `/var/lib/nixos`, `/var/lib/systemd`, `machine-id` and the dhcpcd lease and DUID (`modules/features/base/impermanence`), NetworkManager state (`modules/features/profiles/desktop`).
- **Fresh at every boot, on purpose**: CUPS (with `printing`), power-profiles-daemon and fwupd (with `firmware`). Printers are found again through Avahi, the power profile is the default one, firmware metadata is fetched again.
- **Yours**: declare what your own modules need next to them, in the same option. `sudo doctor` lists what the next boot would erase ([doctor.md](doctor.md)).

Users are immutable (`users.mutableUsers = false`): passwords come from files under `/persist/passwords/`, never from `passwd`.

## Before you start: Secure Boot setup mode

Hosts that import `secure-boot` (the `example` host does) boot through [lanzaboote](https://github.com/nix-community/lanzaboote): the starter generates its own Secure Boot keys and enrolls them in the firmware, alongside Microsoft's (GPU option ROMs and dual boot need them). Enrollment only works while the firmware is in **setup mode**, with its keys cleared: do it before every install, since a reinstall generates new keys.

In the firmware settings, find the Secure Boot page and choose the option that clears or deletes the keys (often "Reset to Setup Mode" or "Clear Secure Boot keys"). Secure Boot is not enforced in setup mode, so an installer ISO still boots. Hosts without `secure-boot` skip this and boot plain systemd-boot.

## Installer ISO

Every host with `disk` gets an ISO that installs it, entirely offline: it carries the host's system already built and its disko script, and evaluates nothing on the machine.

```sh
nix build .#installer-<host>
sudo dd if=result/iso/<host>-installer.iso of=/dev/<usb stick> bs=4M status=progress conv=fsync
```

Rebuild it after changing the host: the ISO installs the system as it was when you built it. The starter publishes no ISO: each one embeds a single host, so build it from your fork.

Boot the stick in UEFI mode. An assistant starts on the first console:

1. It lists the disks with their size and model, leaving out the stick it runs from, and asks which one to erase. It refuses when another disk already carries this layout's partition labels, since the install could format that one instead: unplug it or erase its partitions first.
2. It asks for the disk passphrase twice, then the password of each user of the host twice (hashed with yescrypt into `/persist/passwords/<user>`).
3. It shows the host, the disk with its model and the users, and asks you to type the disk's name to confirm. Nothing is erased before that.
4. It partitions with the host's disko layout, installs the prebuilt system, saves a hardware report in `/persist/facter.json` and offers to reboot. Type the passphrase at boot, then log in.

If you quit it, `install-host` starts it again. Without any question, for scripts and the CI test:

```sh
install-host --disk /dev/nvme0n1 --passphrase-file ./passphrase --password-file alice=./alice-password --yes --no-reboot
```

To keep Windows on the disk, a host with `dual-boot` takes `install-host --beside-windows`: NixOS goes in the free space and shares the Windows ESP ([dual-boot.md](dual-boot.md#windows-on-the-same-disk)).

`--yes` asks nothing: it skips the confirmation, fails before erasing anything when a disk, passphrase or password is missing or empty, and reboots at the end unless `--no-reboot` is set. Without `--yes`, a value left out is asked for.

The hardware report comes from [nixos-facter](https://github.com/nix-community/nixos-facter) and is kept in `/persist/facter.json`. Copy it next to your host file, `git add` it and name it in the host, as the assistant reminds you at the end: nixpkgs' facter module then turns on the initrd kernel modules, graphics, network, Bluetooth and virtualisation support that machine needs.

```nix
facter = ./facter.json;
```

The prebuilt system boots on common hardware (SATA, NVMe and USB disks are in the default initrd); a machine that needs more, such as an unusual disk controller, needs the matching kernel modules in its host `module` (`boot.initrd.availableKernelModules`) and a new ISO. A host that imports `secrets` needs its age key before the first boot: use [disko-install](#by-hand-with-disko-install) for it, or create the key on the installed machine and rebuild ([services.md](services.md#secrets-secrets-opt-in)).

## Try the install in a VM

Before you touch real hardware, run the whole installer of a host in a local QEMU window, from your fork:

```sh
nix run .#try-install -- <host>
```

It builds `.#installer-<host>` and boots it in a machine that looks like a new one: UEFI firmware with Secure Boot in setup mode (the [OVMF](https://github.com/tianocore/tianocore.github.io/wiki/OVMF) build the `<host>-disk` checks use), a software TPM2 ([swtpm](https://github.com/stefanberger/swtpm)) and a blank disk image. The assistant runs as on a USB stick: pick the only disk, type the passphrase and the passwords. At the first boot of the installed system, a host with `secure-boot` enrolls its keys and the TPM2; at the second, the disk unlocks without the passphrase.

The disk image, the firmware variables and the TPM state are kept in `$XDG_STATE_HOME/try-install/<host>/` (`~/.local/state` by default), so running the command again boots the installed system: the disk comes first in the boot order, the ISO only boots while the disk is blank.

| Option | Effect |
|---|---|
| `--disk-size 64G` | size of the disk image when it is created (default `32G`; it grows as it fills) |
| `--reset` | erase the disk image, the firmware variables and the TPM, then install again from scratch |

It needs a graphical session and KVM: it stops when `/dev/kvm` is missing or not writable, since an install with an emulated CPU takes hours. It runs on `x86_64-linux` only. It never takes a device: any argument starting with `/dev/` is refused, since it only installs on its own disk image.

## Remote install with nixos-anywhere

[nixos-anywhere](https://github.com/nix-community/nixos-anywhere) installs over SSH on a machine booted on any Linux with root access, building on your machine. It has no flag for the device, so set it in the host first, with a stable name from `ls -l /dev/disk/by-id/` on the target (kernel names like `/dev/nvme0n1` can swap on a machine with several disks):

```nix
module.disk.device = "/dev/disk/by-id/nvme-Samsung_SSD_990_PRO_2TB_S7DNNJ0X123456A";
```

Then prepare the passphrase and the user password locally, and install:

```sh
read -rsp 'Passphrase: ' passphrase && echo && printf '%s' "$passphrase" > /tmp/secret.key
install -d -m 700 extra/persist/passwords
mkpasswd -m yescrypt | install -m 600 /dev/stdin extra/persist/passwords/<you>
nix run github:nix-community/nixos-anywhere -- --flake .#<host> --disk-encryption-keys /tmp/secret.key /tmp/secret.key --extra-files extra root@<address>
```

## By hand with disko-install

From any NixOS installer ISO, with your fork cloned in `~/nixos`:

1. Write the LUKS passphrase where disko reads it (no trailing newline):

   ```sh
   read -rsp 'Passphrase: ' passphrase && echo
   read -rsp 'Again: ' again && echo
   [ "$passphrase" = "$again" ] && printf '%s' "$passphrase" > /tmp/secret.key
   ```

2. Hash your user's password. Without this file the account has no password and you cannot log in:

   ```sh
   mkpasswd -m yescrypt | install -m 600 /dev/stdin ~/password
   ```

   On a host that imports `secrets`, create its post-quantum age key now (`age-keygen` 1.3 or later: run `nix --experimental-features 'nix-command flakes' develop ~/nixos` first to get the version your flake pins), then follow [services.md](services.md#secrets-secrets-opt-in) to encrypt the host's secrets for the public key it prints before installing:

   ```sh
   age-keygen -pq | install -m 600 /dev/stdin ~/key.txt
   age-keygen -y ~/key.txt > ~/age.pub
   ```

3. Partition, encrypt, format and install on the disk you name, then reboot and type the passphrase:

   ```sh
   sudo nix --experimental-features 'nix-command flakes' run github:nix-community/disko/latest#disko-install -- \
     --flake ~/nixos#<host> --disk main /dev/disk/by-id/<disk> --write-efi-boot-entries \
     --extra-files ~/password /persist/passwords/<you>
   ```

   With `secrets`, add `--extra-files ~/key.txt /persist/var/lib/sops-nix/key.txt`.

## After the first boot

1. On the first boot, the system generates its Secure Boot keys in `/var/lib/sbctl` (kept in `/persist`) and signs the boot files. Reboot once more: systemd-boot enrolls the keys and restarts the machine, now with Secure Boot enforced. Check it:

   ```sh
   bootctl status | grep 'Secure Boot'
   sudo sbctl verify
   ```

   `Secure Boot: enabled (user)` means the firmware only runs what your keys signed. `sbctl verify` lists kernels under `/EFI/nixos/` as unsigned: that is expected, the signed lanzaboote stub checks them by hash.

   On that first boot, `systemd-bless-boot.service` fails with `Can't find boot counter source file`, since lanzaboote has just signed the boot files again under a new name, and `doctor` lists it among the failed units until the next reboot. It is harmless: the next boot blesses the fresh `+3` entry.

   If the machine later refuses to boot with a Secure Boot violation (a firmware update restored the factory keys), reset the firmware to setup mode: systemd-boot enrolls the keys from `/boot/loader/keys/auto` again.

2. The TPM now unlocks the disk. On a host with `secure-boot`, the `tpm2-enroll` service does it at the first boot where Secure Boot is enforced (the boot after the keys are enrolled): you type the passphrase once more, and the next boots open the disk without it. The passphrase stays as a fallback. Check it:

   ```sh
   journalctl -b -u tpm2-enroll
   sudo systemd-cryptenroll /dev/disk/by-partlabel/disk-main-luks
   ```

   The journal says `Enrolled a TPM2 token`, and the list shows a `password` slot and a `tpm2` slot. The service does nothing when the disk already has a TPM2 token, when the machine has no TPM2, or while Secure Boot is not enforced. The token is sealed to PCR 7 (the Secure Boot state) and to PCR 15 at zero, so another boot chain, a live USB included, does not get the key.

   After changing Secure Boot keys or firmware settings, PCR 7 changes and the passphrase is asked at boot. Wipe the stale token and the service enrolls again at the next boot:

   ```sh
   sudo systemd-cryptenroll --wipe-slot=tpm2 /dev/disk/by-partlabel/disk-main-luks
   ```

   To also require a PIN at boot, wipe the token and enroll by hand; the service leaves a disk with a TPM2 token alone:

   ```sh
   sudo systemd-cryptenroll --wipe-slot=tpm2 /dev/disk/by-partlabel/disk-main-luks
   sudo systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs=7+15:sha256=0000000000000000000000000000000000000000000000000000000000000000 --tpm2-with-pin=yes /dev/disk/by-partlabel/disk-main-luks
   ```

   On a host without `secure-boot`, nothing is enrolled automatically: run the same command without `--tpm2-with-pin=yes` yourself, knowing PCR 7 does not protect against another boot chain there, so the TPM only protects a disk taken out of the machine.

## Going back to the previous generation

A host with `disk` writes every new generation to the boot menu with 3 tries ([Automatic Boot Assessment](https://systemd.io/AUTOMATIC_BOOT_ASSESSMENT/): `boot.loader.systemd-boot.bootCounting`, or `boot.lanzaboote.bootCounting.initialTries` with `secure-boot`). Each boot of that entry spends a try; once the boot reaches `boot-complete.target`, `systemd-bless-boot` marks it good and it keeps no counter. An entry whose 3 tries are spent is marked bad, and systemd-boot starts the newest generation that is not bad instead, usually the one before. An entry whose content changes when a new generation is installed (the first one gains the `machine-id` after the first boot) is written again with 3 tries, and blessed again the next time it starts.

With `auto-upgrade`, `boot-complete.target` also requires `systemd-boot-check-no-failures.service`: a boot where a unit failed once the default target is reached is not blessed. Without `auto-upgrade`, nothing waits for `multi-user.target`: a boot counts as good once its local file systems are mounted. See where an entry stands:

```sh
bootctl list
```

An entry whose file name ends in `+<left>` or `+<left>-<done>` is still being tried, `+0-<done>` is bad and skipped, and one with no `+` is good. A bad generation stays in the menu: pick it by hand to look at its journal (`journalctl -b -1`), then fix the cause and let the next upgrade replace it.

`feature-rollback` and `feature-rollback-secure-boot` prove it: they install a machine, add a generation in `boot` mode whose health check, a service required by `boot-complete.target`, fails while `systemd-boot-check-no-failures` is left out of that target and never runs, check that each of its 3 boots spends a try without being blessed, that the next boot starts the generation before, and that this one keeps no counter. They run on the systemd and lanzaboote of the locked nixpkgs, so a regression in boot counting fails them. `feature-rollback-failed-unit` does the same with a service that fails once `multi-user.target` is reached, which `systemd-boot-check-no-failures` catches.
