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
| `@home` | `/home` | yes, unless the host imports `wipe-home`: then emptied at every boot, the previous one kept 7 days in `/.snapshots` ([home.md](home.md#start-home-empty-at-every-boot)) |
| `@snapshots` | `/.snapshots` | yes: hourly snapshots of `@home` and `@persist` ([services.md](services.md#snapshots-btrbk)) |

Every boot, the initrd deletes `@root` and snapshots it again from the empty `@root-blank`. Anything outside `/nix`, `/home`, `/persist` and `/.snapshots` is gone unless it is declared in `environment.persistence."/persist"`: logs, `/var/lib/nixos`, `/var/lib/systemd` and `machine-id` (`modules/system/impermanence`), NetworkManager state (`modules/profiles/desktop`). Declare what your own modules need next to them, in the same option.

Users are immutable (`users.mutableUsers = false`): passwords come from files under `/persist/passwords/`, never from `passwd`.

## Before you start: Secure Boot setup mode

Hosts that import `secure-boot` (the `example` host does) boot through [lanzaboote](https://github.com/nix-community/lanzaboote): the starter generates its own Secure Boot keys and enrolls them in the firmware, alongside Microsoft's (GPU option ROMs and dual boot need them). Enrollment only works while the firmware is in **setup mode**, with its keys cleared: do it before every install, since a reinstall generates new keys.

In the firmware settings, find the Secure Boot page and choose the option that clears or deletes the keys (often "Reset to Setup Mode" or "Clear Secure Boot keys"). Secure Boot is not enforced in setup mode, so the installer ISO still boots. Hosts without `secure-boot` skip this and boot plain systemd-boot.

## Steps

From the NixOS installer ISO, with your fork cloned in `~/nixos` and `disk` set in `modules/hosts/<host>/default.nix`. Prefer a stable name from `ls -l /dev/disk/by-id/` over `/dev/nvme0n1`: on a machine with several disks, kernel names can swap between boots.

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

   On a host that imports `secrets`, create its post-quantum age key now (`age-keygen` 1.3 or later: run `nix --experimental-features 'nix-command flakes' develop ~/nixos` first to get the version your flake pins), then follow [services.md](services.md#secrets-secrets-opt-in) to encrypt the host's secrets for the public key it prints before installing:

   ```sh
   sudo install -d -m 700 /mnt/persist/var/lib/sops-nix
   age-keygen -pq | sudo install -m 600 /dev/stdin /mnt/persist/var/lib/sops-nix/key.txt
   sudo age-keygen -y /mnt/persist/var/lib/sops-nix/key.txt > ~/age.pub
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

6. The TPM now unlocks the disk. On a host with `secure-boot`, the `tpm2-enroll` service does it at the first boot where Secure Boot is enforced (the boot after the keys are enrolled): you type the passphrase once more, and the next boots open the disk without it. The passphrase stays as a fallback. Check it:

   ```sh
   journalctl -b -u tpm2-enroll
   sudo systemd-cryptenroll /dev/disk/by-partlabel/disk-main-luks
   ```

   The journal says `Enrolled a TPM2 token`, and the list shows a `password` slot and a `tpm2` slot. The service needs no input: the initrd links the volume key it just unlocked into root's kernel keyring (`link-volume-key=`), and the service, which runs early in the boot, uses it to add a random temporary keyslot (slot 31), enrolls the TPM through it, then removes the slot and the key, whatever the outcome. A slot 31 left by an interrupted run is wiped at the next boot. It does nothing when the disk already has a TPM2 token, when the machine has no TPM2, or while Secure Boot is not enforced.

   The trade-off: until the disk is enrolled, every boot leaves the volume key readable by root in the kernel keyring for a few seconds, with a 300 s timeout so a rescue boot or a masked service cannot keep it longer. Once a TPM2 token exists, the initrd drops the key before leaving, so it never reaches the running system.

   The token is sealed to PCR 7 and to PCR 15 at zero, as `systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs=7+15:sha256=0000000000000000000000000000000000000000000000000000000000000000` would. PCR 7 binds the key to the Secure Boot state: a boot chain your keys did not sign, a live USB included, does not get it. PCR 15 must still be zero: the initrd extends it with the key of the volume it unlocks (`tpm2-measure-pcr=yes`), so an attacker who swaps in their own LUKS partition and types their own passphrase cannot ask the TPM for the real key afterwards ([filesystem confusion](https://oddlama.org/blog/bypassing-disk-encryption-with-tpm2-unlock/)). The boot entry editor is disabled (`init=/bin/sh` would otherwise skip the login on an unlocked disk).

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

## Tested in CI

`<host>-disk` installs the host on a blank virtual disk with this exact layout, boots it, writes a file in `/`, `/persist` and `/home`, reboots, and checks that only the one in `/` is gone and that `machine-id` is unchanged. It then takes a btrbk snapshot of `/home` and `/persist` and checks it lands in the `@snapshots` subvolume ([services.md](services.md#snapshots-btrbk)).

For an `x86_64-linux` host with `secure-boot`, the same check then boots the installed disk under OVMF in setup mode with an emulated TPM (swtpm): the keys are enrolled, Secure Boot is enforced, the `tpm2-enroll` service enrolls the disk on PCRs 7 and 15 and leaves only the passphrase and TPM2 keyslots, and the next boot unlocks the disk with the TPM alone, nobody typing the passphrase, extends PCR 15 and drops the volume key in the initrd.
