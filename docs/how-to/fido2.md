# FIDO2 security keys (`fido2`)

Plug in a security key (a YubiKey, a SoloKey, a Nitrokey, any FIDO2 or U2F key), touch it, and you are logged in, `sudo` runs, and once enrolled the disk opens at boot. A host that adds `fido2` to its features gets all three; the password and the LUKS passphrase keep working, with or without the key.

```nix
features = [ "disk" "fido2" ];
```

## Log in and sudo with the key

The feature turns on [pam_u2f](https://developers.yubico.com/pam-u2f/) for every PAM service (the console login, greetd, `sudo`, the screen lock) as `sufficient`: a registered key plugged in is enough, otherwise PAM asks for the password as before. A user with no key registered sees no change.

A key is registered once per person, on any machine with `pamu2fcfg` (a host with `fido2` has it):

```sh
pamu2fcfg -n -o pam://nixos-forge -i pam://nixos-forge
```

Touch the key when it blinks, and copy the line it prints into the person's `u2fKeys`, one line per key, so a spare key works too:

```nix
users.<you> = {
  u2fKeys = [ ":owBYz...,pSJ7...,es256,+presence" ];
};
```

Every host with `fido2` that gives you an account then accepts that key: the feature sets the same origin and app ID, `pam://nixos-forge`, on every machine of the fork, and writes the keys of all its users to `/etc/u2f-mappings`. A host without `fido2` ignores `u2fKeys`. The credential is a public key, safe to commit.

To change the policy, set the `security.pam.u2f` options in the host's `module`: `control = "required"` asks for the password and the key (add `settings.nouserok = true` so users without a key still log in), `settings.cue = false` drops the "Please touch the device" line.

## Unlock the disk with the key

Enroll the key on the LUKS volume of `disk`, with the key plugged in:

```sh
sudo fido2-enroll
```

It runs `systemd-cryptenroll --fido2-device=auto` on `/dev/disk/by-partlabel/disk-main-luks` (or the device given as argument), asks for the passphrase, and asks to touch the key. Repeat with each spare key. Then tell the initrd to use it, and switch:

```nix
module.fido2.unlockLuks = true;
```

At boot, the initrd finds the key, asks for a touch, and opens the disk without the passphrase.

> [!WARNING]
> Enroll the key before you set `unlockLuks`: a boot with `unlockLuks` on a disk with no FIDO2 key enrolled stops in the emergency shell.

`unlockLuks` adds `fido2-device=auto` to every LUKS volume, and systemd-cryptsetup stops on a volume with no FIDO2 key enrolled (`No valid FIDO2 token data found`), so the feature never sets it on its own. If it happens, pick the previous generation in the boot menu.

## With the TPM2

`disk` already asks the initrd to try the TPM2, and `secure-boot` enrolls it once Secure Boot is enforced. systemd-cryptsetup tries the TPM2 first, then the FIDO2 key, then the passphrase:

- TPM2 enrolled and the measured boot unchanged: the disk opens on its own, the key is never asked for.
- No TPM2 token yet, or the TPM2 refuses (firmware update, Secure Boot off): the key unlocks.
- Key enrolled but not plugged in: the initrd prints `Security token not present`, waits 30 seconds for it, then asks for the passphrase.

Importing `fido2` without `unlockLuks` changes nothing in the initrd, so TPM2 unlocking stays as it was.

## Recovery

The passphrase stays in its keyslot: `fido2-enroll` adds a key, it never removes one. A lost key is wiped from the disk with `sudo systemd-cryptenroll --wipe-slot=fido2 /dev/disk/by-partlabel/disk-main-luks` (this removes every enrolled key, enroll the others again), and from PAM by deleting its line in `u2fKeys`.

## Check it worked

`sudo -k && sudo true` asks for a touch of the key instead of your password. With `unlockLuks`, the next boot without a TPM2 token asks for a touch, then opens the disk. The `feature-fido2` check proves both in a VM with an emulated key ([reference/ci.md](../reference/ci.md#checks-of-the-modules)).
