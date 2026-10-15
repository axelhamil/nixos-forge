# Install on a machine

The starter is developed and tested in VMs. This page is the path for your fork when you install it on real hardware. **disko wipes the whole disk set in `disk.device`**: read every step before running it.

## What the disk looks like

| Partition | Content |
|---|---|
| ESP, 1 GiB | vfat on `/boot` |
| rest of the disk | LUKS2 `cryptroot`, then btrfs |

btrfs subvolumes, all mounted with `compress=zstd` and `noatime`:

| Subvolume | Mount | Kept across reboots |
|---|---|---|
| `@root` | `/` | no: restored from `@root-blank` at every boot |
| `@nix` | `/nix` | yes |
| `@persist` | `/persist` | yes |
| `@home` | `/home` | yes |

Every boot, the initrd deletes `@root` and snapshots it again from the empty `@root-blank`. Anything outside `/nix`, `/home` and `/persist` is gone unless it is declared in `environment.persistence."/persist"`: logs, `/var/lib/nixos`, `/var/lib/systemd` and `machine-id` (`modules/system/impermanence`), NetworkManager state (`modules/profiles/desktop`). Declare what your own modules need next to them, in the same option.

Users are immutable (`users.mutableUsers = false`): passwords come from files under `/persist/passwords/`, never from `passwd`.

## Before you start: Secure Boot setup mode

Hosts that import `secure-boot` (the `example` host does) boot through [lanzaboote](https://github.com/nix-community/lanzaboote): the starter generates its own Secure Boot keys and enrolls them in the firmware, alongside Microsoft's (GPU option ROMs and dual boot need them). Enrollment only works while the firmware is in **setup mode**, with its keys cleared: do it before every install, since a reinstall generates new keys.

In the firmware settings, find the Secure Boot page and choose the option that clears or deletes the keys (often "Reset to Setup Mode" or "Clear Secure Boot keys"). Secure Boot is not enforced in setup mode, so the installer ISO still boots. Hosts without `secure-boot` skip this and boot plain systemd-boot.

## Steps

From the NixOS installer ISO, with your fork cloned in `~/nixos` and `disk.device` set in `modules/hosts/<host>/hardware.nix`. Prefer a stable name from `ls -l /dev/disk/by-id/` over `/dev/nvme0n1`: on a machine with several disks, kernel names can swap between boots.

1. Write the LUKS passphrase where disko reads it (no trailing newline):

   ```sh
   read -rsp 'Passphrase: ' passphrase && echo
   read -rsp 'Again: ' again && echo
   [ "$passphrase" = "$again" ] && printf '%s' "$passphrase" > /tmp/secret.key
   ```

2. Partition, encrypt, format and mount on `/mnt`:

   ```sh
   sudo nix --experimental-features 'nix-command flakes' run github:nix-community/disko/latest -- --mode destroy,format,mount --flake ~/nixos#<host>
   ```

3. Set your user's password. Without this file the account has no password and you cannot log in:

   ```sh
   sudo install -d -m 700 /mnt/persist/passwords
   mkpasswd -m yescrypt | sudo install -m 600 /dev/stdin /mnt/persist/passwords/<you>
   ```

4. Install, then reboot and type the passphrase:

   ```sh
   sudo nixos-install --flake ~/nixos#<host> --no-root-passwd
   ```

5. On the first boot, the system generates its Secure Boot keys in `/var/lib/sbctl` (kept in `/persist`) and signs the boot files. Reboot once more: systemd-boot enrolls the keys and restarts the machine, now with Secure Boot enforced. Check it:

   ```sh
   bootctl status | grep 'Secure Boot'
   sudo sbctl verify
   ```

   `Secure Boot: enabled (user)` means the firmware only runs what your keys signed. `sbctl verify` lists kernels under `/EFI/nixos/` as unsigned: that is expected, the signed lanzaboote stub checks them by hash.

   If the machine later refuses to boot with a Secure Boot violation (a firmware update restored the factory keys), reset the firmware to setup mode: systemd-boot enrolls the keys from `/boot/loader/keys/auto` again.

6. Let the TPM unlock the disk. The passphrase stays as a fallback:

   ```sh
   sudo systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs=7+15:sha256=0000000000000000000000000000000000000000000000000000000000000000 /dev/disk/by-partlabel/disk-main-luks
   ```

   PCR 7 binds the key to the Secure Boot state: a boot chain your keys did not sign, a live USB included, does not get it. PCR 15 must still be zero: the initrd extends it with the key of the volume it unlocks (`tpm2-measure-pcr=yes`), so an attacker who swaps in their own LUKS partition and types their own passphrase cannot ask the TPM for the real key afterwards ([filesystem confusion](https://oddlama.org/blog/bypassing-disk-encryption-with-tpm2-unlock/)). Add `--tpm2-with-pin=yes` to also require a PIN at boot. The boot entry editor is disabled (`init=/bin/sh` would otherwise skip the login on an unlocked disk). Enroll again after changing Secure Boot keys or firmware settings, or the passphrase is asked at boot. On a host without `secure-boot`, PCR 7 does not protect against another boot chain: the TPM only protects a disk taken out of the machine.

## Tested in CI

`<host>-disk` installs the host on a blank virtual disk with this exact layout, boots it, writes a file in `/`, `/persist` and `/home`, reboots, and checks that only the one in `/` is gone and that `machine-id` is unchanged.

For an `x86_64-linux` host with `secure-boot`, the same check then boots the installed disk under OVMF in setup mode with an emulated TPM (swtpm): the keys are enrolled, Secure Boot is enforced, the disk is enrolled on PCRs 7 and 15, the passphrase is removed, and the next boot unlocks the disk with the TPM alone and extends PCR 15.
