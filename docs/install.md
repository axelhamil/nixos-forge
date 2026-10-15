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

5. Once booted, let the TPM unlock the disk. The passphrase stays as a fallback:

   ```sh
   sudo systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs=7 /dev/disk/by-partlabel/disk-main-luks
   ```

   PCR 7 binds the key to the Secure Boot state. The boot entry editor is disabled (`init=/bin/sh` would otherwise skip the login on an unlocked disk). Until Secure Boot is enforced (lot 4), any boot chain on this machine, a live USB included, gets the key: the TPM only protects a disk taken out of the machine. Skip this step if that is not enough, and enroll again after enabling Secure Boot or changing firmware settings.

## Tested in CI

`<host>-disk` installs the host on a blank virtual disk with this exact layout, boots it, writes a file in `/`, `/persist` and `/home`, reboots, and checks that only the one in `/` is gone and that `machine-id` is unchanged.
