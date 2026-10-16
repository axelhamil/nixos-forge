# nixos-starter

[![check](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml/badge.svg)](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml)

A small NixOS flake to fork and make your own: edit one file of plain values and you have a system that is tested in CI and keeps itself up to date.

It follows the idea of [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim): a readable starting point, not a framework. You fork it, change what you want, and own the result.

## What you get

- **Dendritic layout**: built on [flake-parts](https://flake.parts) and [import-tree](https://github.com/vic/import-tree). Every `.nix` file under `modules/` is a flake-parts module, loaded automatically. Adding a feature means adding a file, no import list to maintain.
- **One file per machine**: a host is plain values (disk, profile, features, apps, users, locale) in `modules/hosts/<name>/default.nix`, every field documented in [docs/options.md](docs/options.md), and any other NixOS setting still possible.
- **Every host built and booted in CI**: a host gets its `nixosConfigurations` entry, a build check and a NixOS VM boot test (systemd, every home-manager user), or an SD card image for a Raspberry Pi, run on a CI runner of its system (`x86_64-linux` or `aarch64-linux`).
- **Weekly lock updates, merged automatically**: every Monday a PR updates `flake.lock` and merges itself once CI is green.
- **Package diff on every PR**: a comment lists the packages added, removed or upgraded per host, `x86_64-linux` and `aarch64-linux` (via [nvd](https://git.sr.ht/~khumba/nvd)).
- **Encrypted disk that forgets**: [disko](https://github.com/nix-community/disko) layout with LUKS (TPM2 unlock, passphrase fallback) and btrfs, `/` wiped at every boot with [impermanence](https://github.com/nix-community/impermanence), `/home` and declared state in `/persist` kept, or `/home` wiped too with opt-in `wipe-home`, keeping only the paths each module and user declares. A CI test installs it, reboots and proves it.
- **Secure Boot, opt-in**: [lanzaboote](https://github.com/nix-community/lanzaboote) with keys generated on the machine and enrolled automatically, then TPM2 unlock bound to the Secure Boot state and to the unlocked volume (PCRs 7 and 15). A CI test boots the installed disk under OVMF with an emulated TPM and proves the disk opens without the passphrase.
- **Hardware modules, opt-in**: firmware and fwupd, PipeWire and Bluetooth (on with the `desktop` profile), NVIDIA (open kernel module), AMD and Intel GPUs, plus [nixos-hardware](https://github.com/NixOS/nixos-hardware) profiles. CI builds the NVIDIA module against every host's kernel, so a weekly update that would break it never merges.
- **Performance profiles you can measure**: `desktop`, `laptop` and `server` profiles (kernel, sched-ext scheduler, power daemon, zram, oomd), each with a VM test, and `nix run .#bench` to compare schedulers, kernels or settings on your own machine. See [docs/performance.md](docs/performance.md).
- **Services without a cloud**: systemd-resolved with no hidden fallback DNS, hourly btrbk snapshots of `/home` and `/persist`, opt-in SSH (keys only, post-quantum key exchange) and opt-in secrets with [sops-nix](https://github.com/Mic92/sops-nix) and a per-machine post-quantum age key. Each one has a VM test. See [docs/services.md](docs/services.md).
- **Dev base**: [nh](https://github.com/nix-community/nh) to switch with a package diff and clean old generations, offline `nix-locate` and `, <command>` with [nix-index-database](https://github.com/nix-community/nix-index-database), and opt-in rootless Docker, libvirt with an emulated TPM, nix-ld and direnv. nh, nix-index, Docker and libvirt each have a VM test. See [docs/dev.md](docs/dev.md).
- **A shell ready to use**: zsh with autosuggestions and highlighting, starship, git with sane defaults and delta, neovim left for your own config, and modern CLI tools (fzf, zoxide, eza, bat, ripgrep, fd), all through home-manager in the XDG layout and covered by a VM test; bring your own dotfiles, a mirror of your home or [stow](https://www.gnu.org/software/stow/) packages, with one import. See [docs/home.md](docs/home.md).
- **A desktop ready to log in**: greetd with tuigreet, a Plymouth splash that asks the LUKS passphrase, and [Hyprland](https://hypr.land) through UWSM with waybar, fuzzel, ghostty, mako, hyprlock, hypridle and cliphist, all colored from one palette you override, gnome-keyring and the GTK portal. A VM test logs in, locks, unlocks and opens a terminal. See [docs/desktop.md](docs/desktop.md).
- **Raspberry Pi 4 and 5**: board modules on [nixos-raspberrypi](https://github.com/nvmd/nixos-raspberrypi) (vendor kernel built from this flake's nixpkgs), a `pi` profile that sends builds to a remote builder, and an SD card image built in CI on an arm runner. See [docs/raspberry-pi.md](docs/raspberry-pi.md).
- **Lint and format in the check**: nixfmt, statix and deadnix run through `nix fmt` and `nix flake check`.

## Requirements

- Nix with flakes enabled (`nix-command` and `flakes` experimental features)
- An x86_64 Linux machine with KVM (`/dev/kvm`) to run the VM test and the VM

## Quick start

1. Fork the repo on GitHub, then clone your fork.

2. Make the example host yours. A machine is one file of plain values, [`modules/hosts/example/default.nix`](modules/hosts/example/default.nix):

   ```nix
   {
     configurations.nixos.example = {
       stateVersion = "26.05";
       disk = "/dev/nvme0n1";
       hardware = [ "common-pc-ssd" "common-cpu-intel-cpu-only" ];

       profile = "desktop";
       features = [ "secure-boot" "plymouth" "hyprland" ];
       apps = [ "firefox" ];

       users.alice = {
         fullName = "Alice Example";
         email = "alice@example.org";
       };
     };
   }
   ```

   Rename the folder and `example` to your host name, `alice` to your login, then set your disk, your [nixos-hardware](https://github.com/NixOS/nixos-hardware) profiles, and `timeZone`, `locale` and `keyboard` (`UTC`, `en_US.UTF-8` and `us` by default). Every field is listed in [docs/options.md](docs/options.md). Delete the hosts you do not need: `example-server` (a headless machine) and `example-pi` (a Raspberry Pi). Keep new files tracked with `git add`: a flake only sees files known to git.

3. Install the pre-push hook, then check, build and run:

   ```sh
   nix develop
   nix fmt
   nix flake check -L
   nix build .#nixosConfigurations.<host>.config.system.build.vm
   ./result/bin/run-<host>-vm
   ```

   `nix develop` installs a pre-push hook that runs in seconds: treefmt, `nix flake check --no-build` (every host evaluates) and actionlint. Builds and boot tests stay in CI. `nix flake check` ends with `all checks passed!`. The VM opens a window; log in with your user and the password `test` (set for the VM only). Nothing touches a physical disk. To install on a real machine, follow [docs/install.md](docs/install.md).

4. To get the CI, auto updates and package diffs on your fork, follow [docs/ci.md](docs/ci.md). GitHub disables workflows on forks until you enable them in the Actions tab.

## Make it yours

Everything a host turns on is a name in its file:

- **`profile`**: the role of the machine, `desktop`, `laptop`, `server` or `pi`.
- **`features`**: any NixOS module of the starter or yours, by name: `secure-boot`, `wipe-home`, `hyprland`, `plymouth`, `docker`, `libvirt`, `ssh`, `secrets`, `nvidia`, `amd-gpu`, `intel-gpu`, `raspberry-pi-5`, ...
- **`apps`**: a module under `modules/apps/<name>` when there is one (it installs and configures the app), else the nixpkgs package of that name. Unfree packages named here are allowed. A user has its own `apps` too.
- **`users.<name>.home`**: the home-manager modules of a user, `shell` by default.
- **`hardware`**: a hardware module of the starter (`nvidia`, `amd-gpu`, `intel-gpu`, `raspberry-pi-5`, ...) or a [nixos-hardware](https://github.com/NixOS/nixos-hardware) profile.
- **`module`**: any other NixOS setting, for what the fields do not cover. The NixOS snippets in the docs go there (`module.services.resolved.settings = ...`) or in a module of yours.

An unknown name stops the evaluation with the list of known ones.

To add an app with its config, add `modules/apps/<name>/default.nix` with its home-manager part (and `apps.<name>.nixos` for a system-wide part), then name it in `apps`. An app named by the host and by a user is imported once. [`modules/apps/firefox`](modules/apps/firefox/default.nix) is the example: telemetry, sponsored content and Pocket off, the profile kept across boots with `wipe-home`.

```nix
{
  apps.<name>.homeManager = {
    programs.<name>.enable = true;
  };
}
```

A module for the whole machine works the same way: publish `flake.modules.nixos.<name>` from any file under `modules/` and name it in `features`. Your own home config goes in a file such as `modules/users/<you>/default.nix`, as `flake.modules.homeManager.<you>`, listed in your `home` (`[ "shell" "<you>" ]`).

## Layout

`modules/` is split into layers, with one folder per tool (`default.nix`, plus other modules, scripts or assets as needed). `helpers/` is the exception: one file per topic. Every `.nix` file under `modules/` is loaded, so adding a file adds the feature and deleting it removes it.

```text
modules/
├── flake/     flake wiring: flake-parts, formatter, host registry with its apps and options reference, dev shell (git hooks, sops, age), CI and user scripts (closure-diff, bench)
├── helpers/   functions shared by flake-parts modules (`config.helpers`)
├── system/    OS base shared by every host: nix (nh, nix-index), home-manager, memory (zram), resolved, disk (with btrbk snapshots), impermanence; opt-in wipe-home, secure-boot, ssh, secrets
├── hardware/  modules a host picks for its machine: firmware, audio, bluetooth, GPUs, Raspberry Pi 4 and 5
├── profiles/  roles a host picks: desktop, laptop, server, pi
├── dev/       development modules a host picks: docker (rootless), libvirt, nix-ld, direnv
├── home/      home-manager modules a user picks: zsh, starship, git, neovim, cli, the shell bundle, stow
├── desktop/   graphical session: greetd, plymouth, theme, hyprland with waybar, fuzzel, ghostty, mako, lock
├── apps/      apps a host or user picks, with their config: firefox
├── users/     your own home config, created when you need one
└── hosts/     one file per machine
```

A function that two modules need goes to `modules/helpers/<topic>.nix` as `helpers.<name> = ...;`, and is read with `inherit (config.helpers) <name>;`.

## Status and roadmap

Working today: a minimal system (systemd-boot, or lanzaboote with Secure Boot on hosts that import `secure-boot`, LUKS and btrfs with `/` wiped at boot and `/home` too on hosts that import `wipe-home`, NetworkManager, systemd-resolved, hourly btrbk snapshots, opt-in SSH and sops-nix secrets, nh and nix-index, opt-in rootless Docker and libvirt, performance profiles (desktop, laptop, server), firmware with fwupd, PipeWire and Bluetooth, one user with home-manager and a ready zsh, git and neovim, greetd, Plymouth and a Hyprland desktop) that builds, installs and boots in a VM, plus a Raspberry Pi 5 host whose SD card image builds in CI,, a headless `example-server` host on plain systemd-boot, each configured in one file of plain values, with the tooling and CI around it.

The shipped lots and the guardrail each one keeps are in [ROADMAP.md](ROADMAP.md).

## Releases

Versions are cut by [release-please](https://github.com/googleapis/release-please) from Conventional Commits: it keeps a release PR open on `main` with the next version and its `CHANGELOG.md` entry, and merging it publishes the release. The starter is in `0.x`: `feat` and breaking changes bump the minor, `fix` the patch, `chore`, `ci` and `docs` do not release. `1.0.0` will be a deliberate decision, not an accident of a breaking change. Release notes live in [GitHub Releases](https://github.com/axelhamil/nixos-starter/releases): read them before merging upstream into your fork.

## License

[MIT](LICENSE)
