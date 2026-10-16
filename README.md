<div align="center">

# nixos-starter

**A NixOS base to fork: one file per machine, every host booted in CI, a shell and a desktop ready on first login.**

[![check](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml/badge.svg)](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml)
[![NixOS unstable](https://img.shields.io/badge/NixOS-unstable-5277C3?logo=nixos&logoColor=white)](https://nixos.org)
[![flake-parts](https://img.shields.io/badge/built%20with-flake--parts-7EBAE4)](https://flake.parts)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

</div>

In the spirit of [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim): a readable starting point, not a framework. Every piece is a module you turn on by name, read in one file and drop by deleting it. The `example` host turns on every feature that needs neither a secret nor specific hardware, `example-server` shows `wipe-home` and `example-pi` a Raspberry Pi, so you see the whole starter; your fork keeps what it wants.

## A machine is one file

```nix
# modules/hosts/my-laptop/default.nix
{
  configurations.nixos.my-laptop = {
    stateVersion = "26.05";
    hardware = [ "framework-13-7040-amd" ];

    profile = "laptop";
    features = [ "disk" "secure-boot" "plymouth" "hyprland" "docker" ];
    apps = [ "firefox" ];

    timeZone = "Europe/Paris";
    keyboard = "fr";

    users.alice = {
      fullName = "Alice Example";
      email = "alice@example.org";
    };
  };
}
```

That is the whole host: it gets a `nixosConfigurations` entry, a build check, a VM boot test and, with `disk`, an installer ISO (`nix build .#installer-my-laptop`), with no import list to keep. The disk it erases is picked at install, never written in the host. Every field is in [docs/options.md](docs/options.md), and `module` takes any other NixOS setting.

## What you get

**A disk that forgets, and opens on its own.** [disko](https://github.com/nix-community/disko) layout with LUKS and btrfs, `/` wiped at every boot with [impermanence](https://github.com/nix-community/impermanence), only declared state kept in `/persist`, hourly btrbk snapshots, and an opt-in empty `/home` at every boot (`wipe-home`). With `secure-boot`, [lanzaboote](https://github.com/nix-community/lanzaboote) enrolls its own keys, then the first boot with Secure Boot enforced enrolls the TPM2 on its own: the disk opens with no passphrase, bound to the Secure Boot state, the passphrase kept as a fallback. `nix build .#installer-<host>` makes an ISO that installs the host offline: pick a disk, type a passphrase and the passwords, reboot. [docs/install.md](docs/install.md)

**A shell worth logging into.** zsh with autosuggestions, highlighting and a shared history, the starship prompt, git with its own developers' defaults and delta, neovim left for your config, and fzf, zoxide, eza, bat, ripgrep, fd, btop, all through home-manager in the XDG layout. Bring your dotfiles as a mirror of your home or as [stow](https://www.gnu.org/software/stow/) packages. [docs/home.md](docs/home.md)

**A desktop to start from.** A Plymouth splash, greetd with tuigreet, and [Hyprland](https://hypr.land) started by UWSM with waybar, fuzzel, kitty, mako, hyprlock, hypridle and cliphist, JetBrainsMono Nerd Font as monospace, each tool with its own look so your theme starts from a clean slate. Firefox ships with telemetry and sponsored content off. [docs/desktop.md](docs/desktop.md)

**Fast by default.** `desktop`, `laptop`, `server` and `pi` profiles tune the kernel, the sched-ext scheduler, power, zram and systemd-oomd, and `nix run .#bench` measures the difference on your own machine. [docs/performance.md](docs/performance.md)

**Runs on your hardware.** Firmware and fwupd, PipeWire, Bluetooth, NVIDIA, AMD and Intel GPUs, every [nixos-hardware](https://github.com/NixOS/nixos-hardware) profile by name, and Raspberry Pi 4 and 5 with an SD card image. [docs/raspberry-pi.md](docs/raspberry-pi.md)

**Services and dev tools without a cloud.** systemd-resolved with no hidden fallback DNS, SSH with keys only and post-quantum key exchange, secrets with [sops-nix](https://github.com/Mic92/sops-nix), [nh](https://github.com/nix-community/nh) to switch with a package diff, offline `nix-locate` and `, <command>`, rootless Docker, libvirt with an emulated TPM, nix-ld and direnv. [docs/services.md](docs/services.md), [docs/dev.md](docs/dev.md)

**Proved in CI, kept up to date.** Every x86_64 host builds and boots in a VM, Raspberry Pi hosts build their SD image, the modules with behavior have their own VM tests, the install test formats a blank disk, reboots, checks what was wiped and unlocks it with an emulated TPM under Secure Boot, and the installer ISO is booted and run on a blank disk. Each PR gets a package diff comment, `flake.lock` updates itself every Monday and merges once green, and a binary cache means CI only builds what changed. [docs/ci.md](docs/ci.md)

## Quick start

You need Nix with flakes, and an x86_64 Linux machine with KVM to run the tests and the VM.

1. **Fork and clone** the repo.
2. **Make the example yours** in [`modules/hosts/example/default.nix`](modules/hosts/example/default.nix): rename the folder and `example` to your host name, `alice` to your login, set your `hardware`, and drop the `features` and `apps` you do not want. Delete `example-server` and `example-pi` if you do not need them, and `git add` new files: a flake only sees what git tracks.
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
| `features` | any NixOS module by name: `disk` (the encrypted layout, on the disk picked at install), `secure-boot`, `wipe-home`, `plymouth`, `hyprland`, `docker`, `libvirt`, `nix-ld`, `direnv`, `ssh`, `secrets`, yours |
| `apps` | an app of `modules/apps/<name>` with its config, else any nixpkgs package (`kdePackages.kate` works, unfree allowed) |
| `hardware` | a starter module (`nvidia`, `amd-gpu`, `raspberry-pi-5`, ...) or a nixos-hardware profile |
| `users.<name>` | `fullName`, `email`, `admin`, `sshKeys`, its own `apps`, and `home`, the home-manager modules (`shell` by default) |
| `module` | anything else, as plain NixOS config |

An unknown name stops the evaluation with the list of known ones.

**Add a module.** Publish `flake.modules.nixos.<name>` from any file under `modules/` and name it in `features`. Another desktop, Plasma or GNOME, plugs in as a sibling of `hyprland`.

**Add an app with its config.** Create `modules/apps/<name>/default.nix` and name it in `apps`; a `check.nix` with `helpers.appCheck` gives it a VM test. [`modules/apps/firefox`](modules/apps/firefox/default.nix) is the example, swap it for the browser you like the same way.

```nix
{
  apps.<name>.homeManager = {
    programs.<name>.enable = true;
  };
}
```

**Add your home config.** A file such as `modules/users/<you>/default.nix` publishing `flake.modules.homeManager.<you>`, listed in your `home` (`[ "shell" "<you>" ]`). Every value the starter sets is a default you override there without `mkForce`.

## Layout

Every `.nix` file under `modules/` is a [flake-parts](https://flake.parts) module, loaded by [import-tree](https://github.com/vic/import-tree): adding a file adds the feature, deleting it removes it.

```text
modules/
├── flake/     flake wiring: host registry and its checks, installer ISO, formatter, dev shell,
│              CI scripts, bench
├── helpers/   functions shared by modules (config.helpers)
├── system/    the OS base: nix, home-manager, resolved; opt-in disk (with btrbk snapshots),
│              wipe-home, secure-boot, ssh, secrets
├── hardware/  firmware, audio, bluetooth, GPUs, Raspberry Pi 4 and 5
├── profiles/  desktop, laptop, server, pi
├── dev/       docker (rootless), libvirt, nix-ld, direnv
├── home/      zsh, starship, git, neovim, cli, the shell bundle, stow
├── desktop/   plymouth, greetd, hyprland, waybar, fuzzel, kitty, mako, lock
├── apps/      apps with their config: firefox
├── users/     your own home config, when you need one
└── hosts/     one file per machine
```

## Releases

Versions come from Conventional Commits through [release-please](https://github.com/googleapis/release-please). The starter is in alpha: it stays in `0.x`, every release is marked as a pre-release, and `1.0.0` will be the first stable one, a deliberate decision: read the [release notes](https://github.com/axelhamil/nixos-starter/releases) before merging upstream into your fork. Shipped lots and the guardrail each one keeps are in [ROADMAP.md](ROADMAP.md).

## License

[MIT](LICENSE)
