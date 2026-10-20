# Services

DNS, SSH, secrets, printing and snapshots: the services a machine runs, with the defaults they ship with and how to change them.

Nothing here depends on a third-party cloud service: DNS comes from your network, secrets are decrypted with a key that stays on the machine, snapshots stay on the disk.

## DNS: systemd-resolved

Every host runs systemd-resolved (`modules/features/base/resolved`). It uses the DNS servers your network announces. Two defaults differ from systemd's, and a host sets either one in its `module` to change it:

- **No fallback DNS.** systemd ships with Cloudflare, Google and Quad9 compiled in as a fallback; `FallbackDNS=` is empty, so queries never leave for a provider you did not choose.
- **LLMNR off.** It answers name lookups from anyone on the local network, the classic target of poisoning attacks.

DNS-over-TLS and DNSSEC are off: both need servers that support them, and strict mode breaks home routers and captive portals. To pick your own resolvers, set them in your host's `module`:

```nix
services.resolved.settings.Resolve = {
  DNS = [ "192.168.1.53#dns.example.org" ];
  Domains = [ "~." ];
  DNSOverTLS = true;
  DNSSEC = true;
};
```

`Domains = [ "~." ]` routes every lookup to that server rather than to the ones your network announces; check with `resolvectl status`. The name after `#` must match the server's TLS certificate: for a resolver with a certificate from your own CA, add the CA to `security.pki.certificateFiles`.

If your fork enables avahi, also set `MulticastDNS = false` there, so the two do not both answer on port 5353.

## SSH: `ssh` (opt-in)

Add `ssh` to the features of a host you log into remotely. It allows public keys only (no password, no keyboard-interactive, no root login) and keeps a single ED25519 host key, in `/persist/etc/ssh` when `/` is wiped at boot so clients do not see a new key after every reboot. Key exchange is limited to the post-quantum `mlkem768x25519-sha256` and `sntrup761x25519-sha512` and to `curve25519-sha256`: the finite-field Diffie-Hellman that OpenSSH 10 stopped offering by default is refused. OpenSSH's own `PerSourcePenalties` already slows down clients that keep failing, so fail2ban is not needed.

Add your public key to your person, in `modules/users/<you>/default.nix`, before the first remote login; it then opens every machine that gives you an account:

```nix
users.<you>.sshKeys = [ "ssh-ed25519 AAAA... you@laptop" ];
```

Root cannot log in, so deploy as yourself: `nixos-rebuild switch --flake .#<host> --target-host <you>@<host> --sudo --ask-sudo-password`.

## Secrets: `secrets` (opt-in)

Add `secrets` to the features of a host to decrypt [sops](https://github.com/getsops/sops) files at boot with [sops-nix](https://github.com/Mic92/sops-nix). Each machine has its own age key in `/persist/var/lib/sops-nix/key.txt` (`/var/lib/sops-nix/key.txt` without `disk`), independent of SSH, so secrets work on hosts without `ssh`. `nix develop` provides `sops` and `age`.

Use post-quantum age keys (`age-keygen -pq`, age 1.3 and later) for every recipient: a file is only as strong as its weakest recipient.

1. Create your own key on your workstation, once: `mkdir -p ~/.config/sops/age && age-keygen -pq -o ~/.config/sops/age/keys.txt`.
2. Create the machine key during the install, as in [install.md](../tutorials/install.md#by-hand-with-disko-install). Its public key is about 2,000 characters: set a password for the installer's user with `passwd`, then fetch it from your workstation with `scp nixos@<installer-ip>:age.pub .`.
3. On your workstation, add both public keys to `.sops.yaml`:

   ```yaml
   keys:
     - &me age1pq1...
     - &laptop age1pq1...
   creation_rules:
     - path_regex: ^modules/hosts/laptop/secrets\.yaml$
       key_groups:
         - age: [*me, *laptop]
   ```

4. Write the secrets with `sops modules/hosts/laptop/secrets.yaml` (or run `sops updatekeys` on an existing file after adding the machine), then declare them in the host's `module`:

   ```nix
   sops.defaultSopsFile = ./secrets.yaml;
   sops.secrets.wifi-password = { };
   ```

5. Commit, pull in the installer's `~/nixos`, then finish the [disko-install](../tutorials/install.md#by-hand-with-disko-install) with the key's `--extra-files`. A machine whose key is missing from a file boots without that file's secrets.

The value is at `/run/secrets/wifi-password` (root only by default; `owner`, `mode` and `restartUnits` per secret). User passwords can move there too, instead of `/persist/passwords`: `sops.secrets.<you>-password.neededForUsers = true;` and `users.users.<you>.hashedPasswordFile = config.sops.secrets.<you>-password.path;`.

## Printing: `printing` (opt-in)

CUPS with driverless printing (IPP Everywhere): network printers are found through Avahi and its mDNS name resolution, with no driver to install. Their state starts fresh at every boot on a host with `disk`, so printers are found again rather than kept; `sudo doctor` does not report it as lost.

## Snapshots: btrbk

Hosts with `disk` snapshot `/home` and `/persist` every hour into the `@snapshots` subvolume, mounted on `/.snapshots` (`modules/features/disk/snapshots`). A snapshot is only taken when the subvolume changed; every snapshot is kept for 2 days, then one per day for 14 days and one per week for 4 weeks. `/` is not snapshotted: it is rebuilt at every boot.

A snapshot sits on the same disk: it undoes a deleted file or a bad edit, it is not a backup. Restore a file by copying it back:

```sh
ls /.snapshots
cp -a /.snapshots/home.20261015T1400/<you>/notes.md ~/notes.md
```

On a host with `wipe-home`, btrbk snapshots `/persist` only: a declared home path is in `/.snapshots/persist.<date>/home/<you>/`, and the rest of each wiped home in `/.snapshots/home-wiped.<date>` for 7 days ([wipe-home.md](../how-to/wipe-home.md)).

Snapshots of `/persist` keep what it held for up to 4 weeks, passwords and the age key included: after rotating a leaked secret, delete the older snapshots too (`sudo btrfs subvolume delete /.snapshots/persist.<date>`).
