# Install without the ISO

Two other ways to install a host with `disk`, for when the [installer ISO](../tutorials/install.md) does not fit: over SSH with nixos-anywhere, or by hand from any NixOS ISO with disko-install. Both erase the whole disk they are given, and both skip the assistant: you name the disk and prepare the passphrase and the passwords yourself.

| Path | When | Picks the disk with |
|---|---|---|
| [nixos-anywhere](#remote-install-with-nixos-anywhere) | a machine reachable over SSH, booted on any Linux with root access | `disk.device` in the host |
| [disko-install](#by-hand-with-disko-install) | by hand, from any NixOS ISO with your fork cloned; also the way to give a host with `secrets` its age key before the first boot | `--disk main <device>` |

On a host with `secure-boot`, first put the firmware in setup mode: [step 2](../tutorials/install.md#step-2-put-the-firmware-in-setup-mode).

After either one, go on with [step 5 of the tutorial](../tutorials/install.md#step-5-check-secure-boot): the first boots are the same.

## Remote install with nixos-anywhere

[nixos-anywhere](https://github.com/nix-community/nixos-anywhere) installs over SSH, building on your machine. It has no flag for the device, so set it in the host first, with a stable name from `ls -l /dev/disk/by-id/` on the target (kernel names like `/dev/nvme0n1` can swap on a machine with several disks):

```nix
module.disk.device = "/dev/disk/by-id/nvme-<model>_<serial>";
```

Then prepare the passphrase and the user password locally, and install:

```sh
read -rsp 'Passphrase: ' passphrase && echo && printf '%s' "$passphrase" > /tmp/secret.key
install -d -m 700 extra/persist/passwords
mkpasswd -m yescrypt | install -m 600 /dev/stdin extra/persist/passwords/<you>
nix run github:nix-community/nixos-anywhere -- --flake .#<host> --disk-encryption-keys /tmp/secret.key /tmp/secret.key --extra-files extra root@<address>
```

**It worked when** the machine reboots, you type the passphrase on its console, and `ssh <you>@<address> bootctl status` answers. Delete `/tmp/secret.key` and `extra/` afterwards.

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

   On a host that imports `secrets`, create its post-quantum age key now (`age-keygen` 1.3 or later: run `nix --experimental-features 'nix-command flakes' develop ~/nixos` first to get the version your flake pins), then follow [secrets.md](secrets.md) to encrypt the host's secrets for the public key it prints before installing:

   ```sh
   age-keygen -pq | install -m 600 /dev/stdin ~/key.txt
   age-keygen -y ~/key.txt > ~/age.pub
   ```

3. Partition, encrypt, format and install on the disk you name:

   ```sh
   sudo nix --experimental-features 'nix-command flakes' run github:nix-community/disko/latest#disko-install -- \
     --flake ~/nixos#<host> --disk main /dev/disk/by-id/<disk> --write-efi-boot-entries \
     --extra-files ~/password /persist/passwords/<you>
   ```

   With `secrets`, add `--extra-files ~/key.txt /persist/var/lib/sops-nix/key.txt`.

4. Reboot and type the passphrase.

**It worked when** you log in as your user; with `secrets`, `ls /run/secrets` lists the host's secrets.
