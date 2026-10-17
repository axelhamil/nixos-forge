<div align="center">

# nixos-starter

**A NixOS base to fork: one file per machine, every host booted in CI, a shell and a desktop ready on first login.**

[![check](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml/badge.svg)](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml)
[![NixOS unstable](https://img.shields.io/badge/NixOS-unstable-5277C3?logo=nixos&logoColor=white)](https://nixos.org)
[![flake-parts](https://img.shields.io/badge/built%20with-flake--parts-7EBAE4)](https://flake.parts)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

</div>

In the spirit of [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim): a readable starting point, not a framework. Every piece is a module you turn on by name, read in one file and drop by deleting it. The commented `example` host lists every profile, feature, hardware module and app, one line each to uncomment, `example-server` shows `wipe-home` and `example-pi` a Raspberry Pi, so you see the whole starter; your fork keeps what it wants.

## A machine is one file

```nix
# modules/hosts/my-laptop/default.nix
{
  configurations.nixos.my-laptop = {
    stateVersion = "26.05";
    hardware = [ "framework-13-7040-amd" ];

    profile = "laptop";
    features = [ "disk" "secure-boot" "plymouth" "docker" ];
    apps = [ "hyprland" "firefox" ];

    timeZone = "Europe/Paris";
    keyboard = "fr";

    users.alice = { };
  };
}
```

`users.alice` gives an account to a person defined once, in `modules/users/alice/default.nix`, with her name, email, SSH keys, apps and own home-manager config; every machine that names her gets the same person.

```nix
{
  users.alice = {
    fullName = "Alice Example";
    email = "alice@example.org";
    apps = [ "shell" "stow" ];
  };
}
```

That is the whole host: it gets a `nixosConfigurations` entry, a build check, a VM boot test and, with `disk`, an installer ISO (`nix build .#installer-my-laptop`), with no import list to keep. The disk it erases is picked at install, never written in the host. Every field is in [docs/options.md](docs/options.md), and `module` takes any other NixOS setting.

## What you get

| | |
|---|---|
| **Disk** | [disko](https://github.com/nix-community/disko) layout with LUKS and btrfs, `/` wiped at every boot with [impermanence](https://github.com/nix-community/impermanence), hourly btrbk snapshots, opt-in empty `/home` at every boot, opt-in hibernation to an encrypted swap file. [install](docs/install.md), [wipe-home](docs/wipe-home.md), [hibernate](docs/hibernate.md) |
| **Boot** | opt-in Secure Boot with [lanzaboote](https://github.com/nix-community/lanzaboote); the disk then unlocks with the TPM2, enrolled on its own, passphrase kept as a fallback. [install](docs/install.md) |
| **Install** | `nix build .#installer-<host>` makes an offline ISO of your host: pick a disk, type a passphrase and the passwords, reboot. [install](docs/install.md) |
| **Shell** | zsh, starship, git with delta, neovim, fzf, zoxide, eza, bat, ripgrep, fd, btop through home-manager in the XDG layout; your dotfiles with [stow](https://www.gnu.org/software/stow/). [apps](docs/apps.md) |
| **Desktop** | Plymouth, greetd with tuigreet, [Hyprland](https://hypr.land) with waybar, fuzzel, kitty, mako, hyprlock and hypridle, Firefox with telemetry off. [desktop](docs/desktop.md) |
| **Performance** | `desktop`, `laptop`, `server` and `pi` profiles (kernel, sched-ext, power, zram, oomd). [performance](docs/performance.md) |
| **Hardware** | firmware, PipeWire, Bluetooth, NVIDIA, AMD and Intel GPUs, any [nixos-hardware](https://github.com/NixOS/nixos-hardware) profile, Raspberry Pi 4 and 5. [raspberry-pi](docs/raspberry-pi.md) |
| **Services and dev** | systemd-resolved, key-only SSH, [sops-nix](https://github.com/Mic92/sops-nix) secrets, [nh](https://github.com/nix-community/nh), offline `nix-locate`, rootless Docker, libvirt, nix-ld, direnv. [services](docs/services.md), [dev](docs/dev.md) |
| **Updates** | every Monday your fork gets a PR with fresh inputs, merged once CI is green, and one with the starter's latest release for you to review; machines pull your fork with `update`, or weekly on their own with `auto-upgrade`. [updates](docs/updates.md) |
| **Doctor** | `sudo doctor` checks Secure Boot, the TPM2 token, snapshots, failed units, disk space and your checkout, and lists what the next boot erases from `/`. [doctor](docs/doctor.md) |
| **CI** | every host builds and boots in a VM, the install is tested on a blank disk, each PR gets a package diff, `flake.lock` updates itself weekly, only what the cache lacks is built. [ci](docs/ci.md) |

## Quick start

You need Nix with flakes, and an x86_64 Linux machine with KVM to run the tests and the VM.

1. **Fork and clone** the repo.
2. **Make it yours** with `nix run .#init`: it asks for your host name, profile, login, name, email, time zone, keyboard, locale, features, hardware and apps, writes `modules/hosts/<host>/default.nix` from the commented [example host](modules/hosts/example/default.nix) and `modules/users/<login>/default.nix` from the commented [example person](modules/users/alice/default.nix), removes the examples you do not keep and stages the result (`--help` lists a flag for every answer, `--yes` asks nothing). To do it by hand, copy the example host to `modules/hosts/<host>` and the example person to `modules/users/<login>`, rename `example` and `alice`, uncomment what you want, and `git add` them: a flake only sees what git tracks.
3. **Check it and boot it** in a VM:

   ```sh
   nix develop        # installs a pre-push hook that runs in seconds
   nix fmt
   nix flake check -L # ends with: all checks passed!
   nix build .#nixosConfigurations.<host>.config.system.build.vm
   ./result/bin/run-<host>-vm
   ```

   Log in with your user and the password `test`, set for the VM only. Nothing touches a physical disk.
4. **Install** on a real machine: build the installer ISO, write it to a USB stick and boot it; the assistant asks for the disk, the passphrase and the passwords. Other paths (nixos-anywhere, disko-install) are in [docs/install.md](docs/install.md). Turn on the CI of your fork with [docs/ci.md](docs/ci.md).

   ```sh
   nix build .#installer-<host>   # result/iso/<host>-installer.iso
   ```

## Make it yours

| Field | What it takes |
|---|---|
| `profile` | the role: `desktop`, `laptop`, `server` or `pi` |
| `features` | any NixOS module by name: `disk` (the encrypted layout, on the disk picked at install), `secure-boot`, `wipe-home`, `plymouth`, `docker`, `libvirt`, `nix-ld`, `ssh`, `secrets`, yours |
| `apps` | an app of `modules/apps/<name>` with its config (`hyprland`, `direnv`, `firefox`, `stow`, ...), else any nixpkgs package (`kdePackages.kate` works, unfree allowed) |
| `hardware` | a starter module (`nvidia`, `amd-gpu`, `raspberry-pi-5`, ...) or a nixos-hardware profile |
| `users.<name>` | an account for the person of `modules/users/<name>`: `admin`, `groups` and `apps` on this machine, added to the person's |
| `facter` | the hardware report the installer saves in `/persist/facter.json`, copied next to the host file: `./facter.json` |
| `flake` | where the machine updates from: your checkout or `github:<you>/<fork>` |
| `module` | anything else, as plain NixOS config |

A person (`modules/users/<name>`, every field in [docs/options.md](docs/options.md#person-options)) has `fullName`, `email`, `sshKeys`, `apps` that follow them on every machine (`shell` by default) and `module`, their own home-manager config.

An unknown name stops the evaluation with the list of known ones; in `apps`, a name that is no app and no nixpkgs package; in `users`, a person with no folder in `modules/users`.

**Add a feature.** Publish `features.<name>` from a file under `modules/features/<name>` and name it in `features`. Another desktop, Plasma or GNOME, plugs in as an app, a sibling of `hyprland`.

**Add an app with its config** ([docs/apps.md](docs/apps.md)). Create `modules/apps/<name>/default.nix` and name it in `apps`; a `check.nix` with `helpers.appCheck` gives it a VM test, `app-<name>`. [`modules/apps/firefox`](modules/apps/firefox/default.nix) is the example, swap it for the browser you like the same way.

```nix
{
  apps.<name>.homeManager = {
    programs.<name>.enable = true;
  };
}
```

**Add your home config.** `users.<you>.module` in `modules/users/<you>`, home-manager config that follows you on every machine ([docs/apps.md](docs/apps.md#change-a-starter-default)). Every setting a starter app writes, its `enable` flags aside, is a default you override there without `mkForce`.

**Change your system.** Edit the checkout, try it with `nh os test`, switch with `nh os switch`, then commit and push so your other machines follow ([docs/updates.md](docs/updates.md#the-edit-loop)).

## Layout

Every `.nix` file under `modules/` is a [flake-parts](https://flake.parts) module, loaded by [import-tree](https://github.com/vic/import-tree): adding a file adds the feature, deleting it removes it.

```text
modules/
├── flake/     flake wiring: host registry and its checks, installer ISO, formatter, dev shell,
│              CI scripts, helpers/ (functions shared by modules, config.helpers)
├── features/  the system: base/ (nix, home-manager, impermanence, resolved, doctor, update),
│              profiles/ (desktop, laptop, server, pi), hardware/ (firmware, audio, GPUs,
│              Raspberry Pi), and disk, wipe-home, secure-boot, ssh, secrets, docker, libvirt,
│              nix-ld, greetd, plymouth, ...
├── apps/      everything a user runs, with its config: hyprland, waybar, fuzzel, kitty, mako,
│              lock, the shell bundle (zsh, starship, git, neovim, cli), direnv, stow, firefox
├── users/     one folder per person: name, email, SSH keys, apps and home config
└── hosts/     one file per machine, giving accounts to people
```

## Releases

Versions come from Conventional Commits through [release-please](https://github.com/googleapis/release-please). The starter is in alpha: it stays in `0.x` and every release is a pre-release, and your fork receives each one as a PR to review every Monday ([docs/updates.md](docs/updates.md)). Shipped lots and their CI guardrails are in [ROADMAP.md](ROADMAP.md).

## License

[MIT](LICENSE)
