# nixos-starter

[![check](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml/badge.svg)](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml)

A small NixOS flake to fork and make your own: rename two things and you have a system that is tested in CI and keeps itself up to date.

It follows the idea of [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim): a readable starting point, not a framework. You fork it, change what you want, and own the result.

## What you get

- **Dendritic layout**: built on [flake-parts](https://flake.parts) and [import-tree](https://github.com/vic/import-tree). Every `.nix` file under `modules/` is a flake-parts module, loaded automatically. Adding a feature means adding a file, no import list to maintain.
- **Every host built and booted in CI**: declare a host in `configurations.nixos.<name>` and it gets its `nixosConfigurations` entry, a build check and a NixOS VM boot test (systemd, every home-manager user), run on a CI runner of its system (`x86_64-linux` or `aarch64-linux`).
- **Weekly lock updates, merged automatically**: every Monday a PR updates `flake.lock` and merges itself once CI is green.
- **Package diff on every PR**: a comment lists the packages added, removed or upgraded per `x86_64-linux` host (via [nvd](https://git.sr.ht/~khumba/nvd)).
- **Encrypted disk that forgets**: [disko](https://github.com/nix-community/disko) layout with LUKS (TPM2 unlock, passphrase fallback) and btrfs, `/` wiped at every boot with [impermanence](https://github.com/nix-community/impermanence), `/home` and declared state in `/persist` kept. A CI test installs it, reboots and proves it.
- **Lint and format in the check**: nixfmt, statix and deadnix run through `nix fmt` and `nix flake check`.

## Requirements

- Nix with flakes enabled (`nix-command` and `flakes` experimental features)
- An x86_64 Linux machine with KVM (`/dev/kvm`) to run the VM test and the VM

## Quick start

1. Fork the repo on GitHub, then clone your fork.

2. Rename the example user `alice` and the example host `example`. Both are hard coded, so this is a manual find and replace:

   ```sh
   git mv modules/users/alice modules/users/<you>
   git mv modules/hosts/example modules/hosts/<host>
   grep -rn -e alice -e example modules
   ```

   Replace every match the `grep` prints: `alice` is the user (account, module name, home-manager), `example` is the host (`configurations.nixos.<host>`, `networking.hostName`). Then set your time zone, locale and keyboard in `modules/hosts/<host>/default.nix` (neutral `UTC`, `en_US.UTF-8` and `us` by default), and adjust `modules/hosts/<host>/hardware.nix` (`disk.device`, CPU microcode) to your machine. Keep new files tracked with `git add`: a flake only sees files known to git.

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

## Layout

`modules/` is split into layers, with one folder per tool (`default.nix`, plus other modules, scripts or assets as needed). `helpers/` is the exception: one file per topic.


```text
modules/
├── flake/     flake wiring: flake-parts, formatter, host registry, CI scripts
├── helpers/   functions shared by flake-parts modules (`config.helpers`)
├── system/    OS base shared by every host: nix, home-manager, disk, impermanence
├── profiles/  roles a host picks: desktop
├── users/     accounts and their home-manager config
├── hosts/     one folder per machine: its registry entry and hardware
├── desktop/   (planned) graphical session
└── apps/      (planned) terminal, shell, editor and other tools
```

Modules publish themselves under `flake.modules.nixos.<name>` or `flake.modules.homeManager.<name>`. A host is declared in `modules/hosts/<host>/default.nix` with `configurations.nixos.<host>`: its `system` (`x86_64-linux` by default), its `module`, which imports the named modules it wants, and an optional `testScript` appended to its boot test.

A function that two modules need goes to `modules/helpers/<topic>.nix` as `helpers.<name> = ...;`, and is read with `inherit (config.helpers) <name>;`. `helpers.hostsOn system` (the hosts of one platform) is the first one.

## Status and roadmap

Working today: a minimal system (systemd-boot, LUKS and btrfs with `/` wiped at boot, NetworkManager, latest kernel, one user with home-manager) that builds, installs and boots in a VM, with the tooling and CI around it. The user and host are example values you rename by hand. The CI matrix is ready for `aarch64-linux` hosts; the starter itself ships none yet.

Next lots (Secure Boot, hardware, performance profiles, services, desktop, Raspberry Pi) and the guardrail each one keeps are in [ROADMAP.md](ROADMAP.md).

## Releases

Versions are cut by [release-please](https://github.com/googleapis/release-please) from Conventional Commits: it keeps a release PR open on `main` with the next version and its `CHANGELOG.md` entry, and merging it publishes the release. The starter is in `0.x`: `feat` and breaking changes bump the minor, `fix` the patch, `chore`, `ci` and `docs` do not release. `1.0.0` will be a deliberate decision, not an accident of a breaking change. Release notes live in [GitHub Releases](https://github.com/axelhamil/nixos-starter/releases): read them before merging upstream into your fork.

## License

[MIT](LICENSE)
