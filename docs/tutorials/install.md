# Install on a machine

From a host file to a real machine that boots, unlocks itself with the TPM2 and rolls back a bad upgrade on its own. Upstream is developed and tested in VMs. This page is the path for your fork when you install it on real hardware.

> [!WARNING]
> A host that lists `disk` in `features` gets the encrypted layout below, and **the install erases the whole disk it is given**. Read every step before running one.

You need:

- your fork, with a host that lists `disk` in its `features` (and `secure-boot` for Secure Boot and the TPM2);
- a USB stick, which writing the ISO erases;
- a machine that boots in UEFI mode.

The registry (your host entries) never names the disk: you pick it during the install. `disk.device` defaults to `/dev/null/no-disk-chosen`, a path that can never exist. So a command run without a device fails instead of erasing a disk. The installed system mounts its partitions by label (`/dev/disk/by-partlabel/disk-main-ESP`, `disk-main-luks`), so the same build fits any disk.

This tutorial uses the installer ISO. To install over SSH with nixos-anywhere, or by hand with disko-install, see [Install without the ISO](../how-to/install-without-iso.md).

## Step 1: try the install in a VM

Before you touch real hardware, run the whole installer of your host in a local QEMU window:

```sh
nix run .#try-install -- <host>
```

It builds the installer ISO of the host and boots it in a machine that looks like a new one:

- UEFI firmware with Secure Boot in setup mode ([OVMF](https://github.com/tianocore/tianocore.github.io/wiki/OVMF), as in the `<host>-disk` checks);
- a software TPM2 ([swtpm](https://github.com/stefanberger/swtpm));
- a blank disk image.

Answer the assistant as you will on the real machine ([step 4](#step-4-run-the-installer)): pick the only disk, type the passphrase and the passwords.

**It worked when** the installed system boots and asks for the passphrase, and, on a host with `secure-boot`, the second boot after that opens the disk without it.

The disk image, the firmware variables and the TPM state are kept in `$XDG_STATE_HOME/try-install/<host>/` (`~/.local/state` by default). Running the command again boots the installed system: the disk comes first in the boot order, and the ISO only boots while the disk is blank.

| Option | Effect |
|---|---|
| `--disk-size 64G` | size of the disk image when it is created (default `32G`; it grows as it fills) |
| `--reset` | erase the disk image, the firmware variables and the TPM, then install again from scratch |

It needs a graphical session and KVM, on `x86_64-linux`: it stops when `/dev/kvm` is missing or not writable, since an install with an emulated CPU takes hours. It never takes a device: any argument starting with `/dev/` is refused.

## Step 2: put the firmware in setup mode

Skip this step on a host without `secure-boot`: it boots plain systemd-boot.

Hosts that import `secure-boot` boot through [lanzaboote](https://github.com/nix-community/lanzaboote). The feature generates its own Secure Boot keys and enrolls them in the firmware, alongside Microsoft's (GPU option ROMs and dual boot need them). Enrollment only works while the firmware is in **setup mode**, with its keys cleared. Do it before every install, since a reinstall generates new keys.

In the firmware settings, find the Secure Boot page and choose the option that clears or deletes the keys (often "Reset to Setup Mode" or "Clear Secure Boot keys"). Secure Boot is not enforced in setup mode, so the installer ISO still boots.

**It worked when** the Secure Boot page shows setup mode, or no platform key.

## Step 3: build the ISO and write it to a USB stick

Every host with `disk` gets an ISO that installs it entirely offline: it carries the host's system already built and its disko script, and evaluates nothing on the machine. Upstream publishes no ISO, since each one embeds a single host: build it from your fork.

```sh
nix build .#installer-<host>
ls -lh result/iso/
```

**It worked when** `ls` lists `<host>-installer.iso`. Find the stick's device with `lsblk` (plug it in and look for the new disk of its size), then write the ISO to it:

```sh
sudo dd if=result/iso/<host>-installer.iso of=/dev/<usb stick> bs=4M status=progress conv=fsync
lsblk /dev/<usb stick>
```

**It worked when** `lsblk` shows partitions on the stick. Rebuild the ISO after changing the host: it installs the system as it was when you built it.

## Step 4: run the installer

Boot the stick in UEFI mode. An assistant starts on the first console:

1. It lists the disks with their size and model, leaving out the stick it runs from, and asks which one to erase. It refuses when another disk already carries this layout's partition labels, since the install could format that one instead. Unplug it or erase its partitions first.
2. It asks for the disk passphrase twice, then the password of each user of the host twice (hashed with yescrypt into `/persist/passwords/<user>`).
3. It shows the host, the disk with its model and the users, and asks you to type the disk's name to confirm. Nothing is erased before that.
4. It partitions with the host's disko layout, installs the prebuilt system, saves a hardware report in `/persist/facter.json` and offers to reboot.

**It worked when** the machine reboots from its disk, asks for the passphrase, and you log in as your user.

If you quit the assistant, `install-host` starts it again. To keep Windows on the disk, a host with `dual-boot` takes `install-host --beside-windows`: NixOS goes in the free space and shares the Windows ESP ([dual-boot.md](../how-to/dual-boot.md#windows-on-the-same-disk)).

For scripts and the CI test, `install-host` asks nothing with `--yes`:

```sh
install-host --disk /dev/nvme0n1 --passphrase-file ./passphrase --password-file <you>=./password --yes --no-reboot
```

`--yes` skips the confirmation, fails before erasing anything when a disk, passphrase or password is missing or empty, and reboots at the end unless `--no-reboot` is set. Without `--yes`, a value left out is asked for.

The prebuilt system boots on common hardware: SATA, NVMe and USB disks are in the default initrd. A machine that needs more, such as an unusual disk controller, needs two things:

- the matching kernel modules in its host `module` (`boot.initrd.availableKernelModules`);
- a new ISO.

A host that imports `secrets` needs its age key before the first boot. Install it [with disko-install](../how-to/install-without-iso.md#by-hand-with-disko-install), or create the key on the installed machine and rebuild ([services.md](../reference/services.md#secrets-secrets-opt-in)).

## Step 5: check Secure Boot

On a host with `secure-boot`, the first boot generates the Secure Boot keys in `/var/lib/sbctl` (kept in `/persist`) and signs the boot files. Reboot once more: systemd-boot enrolls the keys and restarts the machine, now with Secure Boot enforced. Then:

```sh
bootctl status | grep 'Secure Boot'
sudo sbctl verify
```

**It worked when** `bootctl` prints `Secure Boot: enabled (user)`: the firmware only runs what your keys signed. `sbctl verify` lists kernels under `/EFI/nixos/` as unsigned. That is expected: the signed lanzaboote stub checks them by hash.

On that first boot, `systemd-bless-boot.service` fails with `Can't find boot counter source file`: lanzaboote has just signed the boot files again under a new name. `doctor` lists it among the failed units until the next reboot. It is harmless, and the next boot blesses the fresh `+3` entry.

If the machine later refuses to boot with a Secure Boot violation (a firmware update restored the factory keys), reset the firmware to setup mode: systemd-boot enrolls the keys from `/boot/loader/keys/auto` again.

## Step 6: check the TPM2 unlocks the disk

On a host with `secure-boot`, the `tpm2-enroll` service enrolls the TPM2 at the first boot where Secure Boot is enforced (the boot after the keys are enrolled). You type the passphrase once more, and the next boots open the disk without it. The passphrase stays as a fallback.

```sh
journalctl -b -u tpm2-enroll
sudo systemd-cryptenroll /dev/disk/by-partlabel/disk-main-luks
```

**It worked when** the journal says `Enrolled a TPM2 token`, the list shows a `password` slot and a `tpm2` slot, and the next boot asks for no passphrase.

The service does nothing when the disk already has a TPM2 token, when the machine has no TPM2, or while Secure Boot is not enforced. The token is sealed to PCR 7 (the Secure Boot state) and to PCR 15 at zero. A PCR is a TPM register that records what booted, so another boot chain, a live USB included, does not get the key.

After changing Secure Boot keys or firmware settings, PCR 7 changes and the passphrase is asked at boot. Wipe the stale token, and the service enrolls again at the next boot:

```sh
sudo systemd-cryptenroll --wipe-slot=tpm2 /dev/disk/by-partlabel/disk-main-luks
```

To also require a PIN at boot, wipe the token and enroll by hand; the service leaves a disk with a TPM2 token alone:

```sh
sudo systemd-cryptenroll --wipe-slot=tpm2 /dev/disk/by-partlabel/disk-main-luks
sudo systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs=7+15:sha256=0000000000000000000000000000000000000000000000000000000000000000 --tpm2-with-pin=yes /dev/disk/by-partlabel/disk-main-luks
```

On a host without `secure-boot`, nothing is enrolled automatically. Run the same command without `--tpm2-with-pin=yes` yourself, knowing PCR 7 does not protect against another boot chain there: the TPM only protects a disk taken out of the machine.

## Step 7: add the hardware report

The installer saved a [nixos-facter](https://github.com/nix-community/nixos-facter) report of the machine in `/persist/facter.json`. Copy it next to your host file in your fork, `git add` it and name it in the host:

```nix
facter = ./facter.json;
```

nixpkgs' facter module then turns on the initrd kernel modules, graphics, network, Bluetooth and virtualisation support that machine needs. Rebuild with `nh os switch` (or `nixos-rebuild switch`).

**It worked when** `sudo doctor` reports no failed unit ([doctor.md](../how-to/doctor.md)).

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
| `@home` | `/home` | yes, unless the host imports `wipe-home`: then emptied at every boot ([wipe-home.md](../how-to/wipe-home.md)) |
| `@snapshots` | `/.snapshots` | yes: hourly snapshots of `@home` and `@persist` ([services.md](../reference/services.md#snapshots-btrbk)) |
| `@swap` | `/swap` | yes: only with `hibernate`, a swap file for the hibernation image, never snapshotted ([hibernate.md](../how-to/hibernate.md)) |

Every boot, the initrd deletes `@root` and snapshots it again from the empty `@root-blank`. Anything outside `/nix`, `/home`, `/persist` and `/.snapshots` is gone unless it is declared in `environment.persistence."/persist"`:

- **Kept by the shipped modules**: logs, `/var/lib/nixos`, `/var/lib/systemd`, `machine-id` and the dhcpcd lease and DUID (`modules/features/base/impermanence`), NetworkManager state (`modules/features/profiles/desktop`).
- **Fresh at every boot, on purpose**: CUPS (with `printing`), power-profiles-daemon and fwupd (with `firmware`). Printers are found again through Avahi, the power profile is the default one, firmware metadata is fetched again.
- **Yours**: declare what your own modules need next to them, in the same option. `sudo doctor` lists what the next boot would erase ([doctor.md](../how-to/doctor.md)).

Users are immutable (`users.mutableUsers = false`): passwords come from files under `/persist/passwords/`, never from `passwd`.

## Going back to the previous generation

A host with `disk` writes every new generation to the boot menu with 3 tries ([Automatic Boot Assessment](https://systemd.io/AUTOMATIC_BOOT_ASSESSMENT/): `boot.loader.systemd-boot.bootCounting`, or `boot.lanzaboote.bootCounting.initialTries` with `secure-boot`).

- Each boot of that entry spends a try. Once the boot reaches `boot-complete.target`, `systemd-bless-boot` marks it good and it keeps no counter.
- An entry whose 3 tries are spent is marked bad, and systemd-boot starts the newest generation that is not bad instead, usually the one before.
- An entry whose content changes when a new generation is installed (the first one gains the `machine-id` after the first boot) is written again with 3 tries, and blessed again the next time it starts.

With `auto-upgrade`, `boot-complete.target` also requires `systemd-boot-check-no-failures.service`: a boot where a unit failed once the default target is reached is not blessed. Without `auto-upgrade`, nothing waits for `multi-user.target`: a boot counts as good once its local file systems are mounted. See where an entry stands:

```sh
bootctl list
```

An entry's file name tells its state:

- `+<left>` or `+<left>-<done>`: still being tried;
- `+0-<done>`: bad, and skipped;
- no `+`: good.

A bad generation stays in the menu: pick it by hand to look at its journal (`journalctl -b -1`), then fix the cause and let the next upgrade replace it. The `feature-rollback` checks prove this path on every change ([reference/ci.md](../reference/ci.md#checks-of-the-modules)).
