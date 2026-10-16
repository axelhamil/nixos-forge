# Services

Nothing here depends on a third-party cloud service: DNS comes from your network, secrets are decrypted with a key that stays on the machine, snapshots stay on the disk.

## DNS: systemd-resolved

Every host runs systemd-resolved (`modules/system/resolved`). It gets the DNS servers your network announces from NetworkManager on `desktop` and `laptop`, and from dhcpcd through systemd's `resolvconf` on other hosts. Two defaults differ from systemd's:

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

`system-resolved` checks the stub resolver answers and that no fallback server or LLMNR is active.

## SSH: `ssh` (opt-in)

Add `ssh` to the features of a host you log into remotely. It allows public keys only (no password, no keyboard-interactive, no root login) and keeps a single ED25519 host key, in `/persist/etc/ssh` when `/` is wiped at boot so clients do not see a new key after every reboot. Key exchange is limited to the post-quantum `mlkem768x25519-sha256` and `sntrup761x25519-sha512` and to `curve25519-sha256`: the finite-field Diffie-Hellman that OpenSSH 10 stopped offering by default is refused. OpenSSH's own `PerSourcePenalties` already slows down clients that keep failing, so fail2ban is not needed.

Add your public key to your user before the first remote login:

```nix
users.<you>.sshKeys = [ "ssh-ed25519 AAAA... you@laptop" ];
```

A hardware security key keeps the private key off the disk: `ssh-keygen -t ed25519-sk` creates one bound to a FIDO2 token.

Root cannot log in, so deploy as yourself: `nixos-rebuild switch --flake .#<host> --target-host <you>@<host> --sudo --ask-sudo-password`.

`system-ssh` logs in from a second VM with a key, checks the post-quantum key exchange is used and the old one refused, and checks password and root logins are refused.

## Secrets: `secrets` (opt-in)

Add `secrets` to the features of a host to decrypt [sops](https://github.com/getsops/sops) files at boot with [sops-nix](https://github.com/Mic92/sops-nix). Each machine has its own age key in `/persist/var/lib/sops-nix/key.txt` (`/var/lib/sops-nix/key.txt` without `disk`), independent of SSH, so secrets work on hosts without `ssh`. `nix develop` provides `sops` and `age`.

Use post-quantum age keys (`age-keygen -pq`, age 1.3 and later) for every recipient: the encrypted files live in a git repository that may be public, and a classic X25519 recipient would let someone who stores them today decrypt them once quantum computers can. A file is only as strong as its weakest recipient. Each post-quantum recipient adds about 1.5 KB to every file: prefer one key per machine and per person over many.

1. Create your own key on your workstation, once: `mkdir -p ~/.config/sops/age && age-keygen -pq -o ~/.config/sops/age/keys.txt`.
2. Create the machine key during the install, before `nixos-install` (step 3 of [install.md](install.md#steps)). A post-quantum public key is about 2,000 characters, too long to copy by hand: set a password for the installer's user with `passwd` (the installer runs sshd), then fetch the `~/age.pub` that step wrote from your workstation with `scp nixos@<installer-ip>:age.pub .`.
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

5. Commit, pull in the installer's `~/nixos`, then run `nixos-install`. A machine whose key is missing from a file boots without that file's secrets.

The value is at `/run/secrets/wifi-password` (root only by default; `owner`, `mode` and `restartUnits` per secret). User passwords can move there too, instead of `/persist/passwords`: `sops.secrets.<you>-password.neededForUsers = true;` and `users.users.<you>.hashedPasswordFile = config.sops.secrets.<you>-password.path;`.

`system-secrets` decrypts a fixture with a post-quantum test key committed in `modules/system/secrets/test/`; that key protects nothing else, never reuse it.

A TPM-sealed age key (`age-plugin-tpm` through `sops.age.plugins`) is possible but not used: its recipients rely on P-256, which is not post-quantum and would weaken every file it is added to, and a TPM reset or a new motherboard would lose every secret. The starter keeps a key file, already protected by the TPM-sealed disk.

## Snapshots: btrbk

Hosts with `disk` snapshot `/home` and `/persist` every hour into the `@snapshots` subvolume, mounted on `/.snapshots` (`modules/system/snapshots`). A snapshot is only taken when the subvolume changed; every snapshot is kept for 2 days, then one per day for 14 days and one per week for 4 weeks. `/` is not snapshotted: it is rebuilt at every boot.

A snapshot sits on the same disk: it undoes a deleted file or a bad edit, it is not a backup. Restore a file by copying it back:

```sh
ls /.snapshots
cp -a /.snapshots/home.20261015T1400/<you>/notes.md ~/notes.md
```

On a host with `wipe-home`, btrbk snapshots `/persist` only: a declared home path is in `/.snapshots/persist.<date>/home/<you>/`, and the rest of each wiped home in `/.snapshots/home-wiped.<date>` for 7 days ([home.md](home.md#start-home-empty-at-every-boot)).

Snapshots of `/persist` keep what it held for up to 4 weeks, passwords and the age key included: after rotating a leaked secret, delete the older snapshots too (`sudo btrfs subvolume delete /.snapshots/persist.<date>`).

A machine installed before `@snapshots` existed creates it once, before switching: `sudo mount -o subvol=/ /dev/mapper/cryptroot /mnt && sudo btrfs subvolume create /mnt/@snapshots && sudo umount /mnt`. Until then the mount is skipped (`nofail`), the machine boots, and `btrbk-snapshots.service` fails visibly (`systemctl --failed`) instead of snapshotting into the wiped `/`.

`<host>-disk` takes a snapshot during the install test and checks it holds the files and sits in its own subvolume, outside the wiped `@root`.
