# nixos-starter

[![check](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml/badge.svg)](https://github.com/axelhamil/nixos-starter/actions/workflows/check.yml)

A small NixOS flake to fork and make your own: rename two things and you have a system that is tested in CI and keeps itself up to date.

It follows the idea of [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim): a readable starting point, not a framework. You fork it, change what you want, and own the result.

## What you get

- **Dendritic layout**: built on [flake-parts](https://flake.parts) and [import-tree](https://github.com/vic/import-tree). Every `.nix` file under `modules/` is a flake-parts module, loaded automatically. Adding a feature means adding a file, no import list to maintain.
- **Every host built and booted in CI**: each host in `nixosConfigurations` gets a build check, and `desktop` boots in a NixOS VM test (systemd, home-manager, user account).
- **Weekly lock updates, merged automatically**: every Monday a PR updates `flake.lock` and merges itself once CI is green.
- **Package diff on every PR**: a comment lists the packages added, removed or upgraded per host (via [nvd](https://git.sr.ht/~khumba/nvd)).
- **Lint and format in the check**: nixfmt, statix and deadnix run through `nix fmt` and `nix flake check`.

## Requirements

- Nix with flakes enabled (`nix-command` and `flakes` experimental features)
- An x86_64 Linux machine with KVM (`/dev/kvm`) to run the VM test and the VM

## Quick start

1. Fork the repo on GitHub, then clone your fork.

2. Rename the example user and host. Both are hard coded today, so this is a manual find and replace:

   ```sh
   git mv modules/users/axel modules/users/<you>
   git mv modules/hosts/desktop modules/hosts/<host>
   grep -rn -e axel -e desktop modules
   ```

   Replace every match the `grep` prints (user name, module names, `networking.hostName`, the test name and its `home-manager-axel.service`). Also adjust `modules/system/locale` (French locale and keyboard by default) and `modules/hosts/<host>/hardware.nix` (disk labels, CPU microcode) to your machine. Keep new files tracked with `git add`: a flake only sees files known to git.

3. Install the pre-push hook, then check, build and run:

   ```sh
   nix develop
   nix fmt
   nix flake check -L
   nix build .#nixosConfigurations.<host>.config.system.build.vm
   ./result/bin/run-<host>-vm
   ```

   `nix develop` installs a pre-push hook that runs in seconds: treefmt, `nix flake check --no-build` (every host evaluates) and actionlint. Builds and boot tests stay in CI. `nix flake check` ends with `all checks passed!`. The VM opens a window; log in with your user and the password `test` (set for the VM only). Nothing touches a physical disk.

4. To get the CI, auto updates and package diffs on your fork, follow [docs/ci.md](docs/ci.md). GitHub disables workflows on forks until you enable them in the Actions tab.

## Layout

`modules/` is split into layers, with one folder per tool (`default.nix`, plus other modules, scripts or assets as needed):

```text
modules/
├── flake/     flake wiring: flake-parts, formatter, checks, CI scripts
├── system/    OS base shared by every host: nix, locale, home-manager
├── users/     accounts and their home-manager config
├── hosts/     one folder per machine: the modules it assembles, hardware, VM test
├── desktop/   (planned) graphical session
└── apps/      (planned) terminal, shell, editor and other tools
```

Modules publish themselves under `flake.modules.nixos.<name>` or `flake.modules.homeManager.<name>`, and a host picks the ones it wants in `modules/hosts/<host>/default.nix`.

## Status and roadmap

Working today: a minimal system (systemd-boot, NetworkManager, latest kernel, one user with home-manager) that builds and boots in a VM, with the tooling and CI around it. The user and host are example values you rename by hand, and only `x86_64-linux` is covered.

Next lots (generic starter, disk with LUKS and impermanence, Secure Boot, hardware, performance profiles, services, desktop, Raspberry Pi) and the guardrail each one keeps are in [ROADMAP.md](ROADMAP.md).

## Releases

Versions are cut by [release-please](https://github.com/googleapis/release-please) from Conventional Commits: it keeps a release PR open on `main` with the next version and its `CHANGELOG.md` entry, and merging it publishes the release. The starter is in `0.x`: `feat` and breaking changes bump the minor, `fix` the patch, `chore`, `ci` and `docs` do not release. `1.0.0` will be a deliberate decision, not an accident of a breaking change. Release notes live in [GitHub Releases](https://github.com/axelhamil/nixos-starter/releases): read them before merging upstream into your fork.

## License

[MIT](LICENSE)
