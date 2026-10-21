# Secrets (`secrets`)

Keep a Wi-Fi password, an API token or a user's password hash in your fork, encrypted, and have each machine decrypt its own at boot. A host that adds `secrets` to its features decrypts [sops](https://github.com/getsops/sops) files with [sops-nix](https://github.com/Mic92/sops-nix), using an age key that stays on the machine.

```nix
features = [ "disk" "secrets" ];
```

This page creates the keys, writes `.sops.yaml`, encrypts a file, uses a secret from a module, and adds a machine later. Run every command on your workstation from `nix develop` in your fork, which provides `sops` and `age` at the versions the flake pins, unless the step says it runs on the machine.

## The keys

Every file is encrypted for a list of age public keys, its recipients, and each holder of a matching private key can decrypt it. You need two kinds:

- **Yours**, on your workstation, to write and edit the files. sops reads it from `~/.config/sops/age/keys.txt`.
- **One per machine**, read by sops-nix at boot from `/persist/var/lib/sops-nix/key.txt` (`/var/lib/sops-nix/key.txt` on a host without `disk`, such as a Pi, where nothing is wiped). The feature sets this path from `impermanence.persistPrefix`, because the secrets are decrypted during activation, before the bind mounts of `/persist` exist, and the key survives the wipe of `/`.

The machine key is a plain age key, independent of SSH: the feature turns off sops-nix's SSH host key fallback (`sops.age.sshKeyPaths = [ ]`), so a host without `ssh` decrypts its secrets too, and an SSH host key is never converted into an age key. Nothing in the flake generates the machine key for you; you create it with `age-keygen` as below.

Use post-quantum keys (`age-keygen -pq`, age 1.3 and later) for every recipient: a file is only as strong as its weakest recipient. A post-quantum public key is about 2,000 characters long.

## Create your key

Once, on your workstation:

```sh
mkdir -p ~/.config/sops/age
age-keygen -pq -o ~/.config/sops/age/keys.txt
```

It prints the public key (`age1pq1...`); `age-keygen -y ~/.config/sops/age/keys.txt` prints it again later. Back up the private key somewhere safe: without it, or a machine key, nobody can edit the files.

## Create the machine key

Create it where the machine will read it, then copy its public key to your workstation.

**During a disko-install**, before installing: [install-without-iso.md](install-without-iso.md#by-hand-with-disko-install) creates `~/key.txt` and `~/age.pub` on the installer and copies the key to `/persist/var/lib/sops-nix/key.txt` with `--extra-files`. Fetch the public key with `scp nixos@<installer-ip>:age.pub .` (set a password for the installer's user with `passwd` first).

**On a machine already running**, with the installer ISO or any other install:

```sh
sudo install -d -m 700 /persist/var/lib/sops-nix
nix shell nixpkgs#age --command sh -c 'age-keygen -pq | sudo install -m 400 /dev/stdin /persist/var/lib/sops-nix/key.txt'
sudo cat /persist/var/lib/sops-nix/key.txt | nix shell nixpkgs#age --command age-keygen -y
```

Drop `/persist` from the paths on a host without `disk`. The last command prints the public key to copy into `.sops.yaml`. The installer ISO does not write this key for you: create it after the first boot like this, then encrypt the host's file for it.

## Write `.sops.yaml`

At the root of your fork, `.sops.yaml` says which recipients each file is encrypted for. Name every key once with an anchor, then give each file its list:

```yaml
keys:
  - &me age1pq1...
  - &laptop age1pq1...
creation_rules:
  - path_regex: ^modules/hosts/laptop/secrets\.yaml$
    key_groups:
      - age: [*me, *laptop]
```

Keep one file per host, next to its host file, so a machine can only decrypt its own secrets. A secret several machines share goes in a file of its own, whose rule lists all of them. The `.sops.yaml` of the fork you started from has one rule, for the test file of the `feature-secrets` check; add yours below it.

## Create and edit an encrypted file

```sh
sops modules/hosts/laptop/secrets.yaml
```

sops opens your `$EDITOR` on the decrypted content and encrypts it again when you save and quit. Write one key per secret:

```yaml
wifi-password: hunter2
api-token: abc123
```

The same command edits the file later. The values are encrypted, the key names stay readable. Stage the file with `git add` before any `nix` command: a flake only sees files tracked by git.

## Use a secret from a module

Declare the file and each secret in the host's `module` (`modules/hosts/laptop/default.nix`):

```nix
module = {
  sops.defaultSopsFile = ./secrets.yaml;
  sops.secrets.wifi-password = { };
};
```

At activation, sops-nix decrypts each declared secret to `/run/secrets/<name>`, owned by root with mode `0400`; the encrypted file is copied to the Nix store, the decrypted value never is. Point a service at that path, or at `config.sops.secrets.wifi-password.path`. Per secret, `owner`, `group` and `mode` give it to a service's user, `restartUnits` restarts units when the value changes, and `sopsFile` reads it from another file than `defaultSopsFile`; every option is in the [sops-nix README](https://github.com/Mic92/sops-nix#readme).

A user's password can move there too, instead of `/persist/passwords/<you>`. Put the hash from `mkpasswd -m yescrypt` in the file as `<you>-password`, then:

```nix
sops.secrets.<you>-password.neededForUsers = true;
users.users.<you>.hashedPasswordFile = config.sops.secrets.<you>-password.path;
```

## Add a machine or rotate the keys

To give a new machine a file it should read, add its public key to the anchors and to the file's rule in `.sops.yaml`, then encrypt the file again for the new list:

```sh
sops updatekeys modules/hosts/laptop/secrets.yaml
```

It shows the recipients it adds or removes and asks before writing. Removing a key from the rule and running it again stops future versions of the file from opening with that key. A key that leaked could still open the old versions in the git history, so also change the secrets themselves, and run `sops rotate -i <file>` to change the file's data key. Snapshots of `/persist` keep the old machine key and passwords for up to 4 weeks ([services.md](../reference/services.md#snapshots-btrbk)).

Commit, push, and let the machine take the new version (`update`, `auto-upgrade` or `nixos-rebuild switch`): the secrets are decrypted again at each activation. A machine whose key is missing from a file boots without that file's secrets.

## Check it worked

On your workstation, `sops decrypt modules/hosts/laptop/secrets.yaml` prints the file with your key. On the machine, after a switch or a reboot:

```sh
sudo ls -l /run/secrets/
sudo cat /run/secrets/wifi-password
sudo test -s /persist/var/lib/sops-nix/key.txt && echo key present
```

A secret missing from `/run/secrets` means the machine key is not among the file's recipients, or the file was not committed: the output of `nixos-rebuild switch` shows the error sops-nix met during activation. The `feature-secrets` check proves the chain in a VM: it installs a key under `/persist`, decrypts a test file at boot, and checks the secret's content and its `400` root mode ([reference/ci.md](../reference/ci.md#checks-of-the-modules)).
