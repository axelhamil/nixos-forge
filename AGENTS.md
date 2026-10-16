# AGENTS.md

**This file is the source of truth for agent instructions.** Claude Code (2.1.277 and later), Cursor, Codex and other tools that follow the AGENTS.md convention read it directly. Add agent guidance here and nowhere else, and do not add a `CLAUDE.md`: Claude Code reads `AGENTS.md` only when no `CLAUDE.md` exists.

## What this is

A NixOS flake meant to be forked (the kickstart.nvim model): a readable starting point, not a framework. Forks rename the example user and host, then grow their own system. Desktops and Raspberry Pi hosts are both in scope.

## Commands

| Goal | Command |
|---|---|
| Install the pre-push hook (once per clone), get `sops` and `age` | `nix develop` |
| Run the pre-push hook by hand | `nix develop -c pre-commit run --hook-stage pre-push --all-files` |
| Format and lint (nixfmt, statix, deadnix) | `nix fmt` |
| All checks: lint, host builds, boot tests | `nix flake check -L` |
| One check | `nix build -L .#checks.x86_64-linux.<name>` |
| Checks the CI matrix runs, with system and runner | `nix eval --json .#ci.matrix` |
| Build the VM | `nix build .#nixosConfigurations.<host>.config.system.build.vm` |
| Boot the VM headless (blocks until killed) | `QEMU_KERNEL_PARAMS=console=ttyS0 ./result/bin/run-<host>-vm -nographic` |
| Package diff against a revision | `nix run .#closure-diff -- <rev>` |
| Benchmark this machine, compare two runs | `nix run .#bench -- run <label>`, `nix run .#bench -- compare <a.json> <b.json>` |
| Lint workflows | `nix run nixpkgs#actionlint` |

A flake only sees files tracked by git: `git add` new files (staging is enough) before any `nix` command, or they are silently missing.

## Architecture: the dendritic pattern

The flake is built with flake-parts, and `flake.nix` hands the whole `modules/` tree to `import-tree`. Consequences:

- **Every `.nix` file under `modules/` is a flake-parts module**, imported automatically. There is no import list: adding a file adds the feature, deleting it removes it. A path segment starting with `_` is ignored by import-tree.
- **NixOS and home-manager config is published, not imported.** A file contributes to `flake.modules.nixos.<name>` or `flake.modules.homeManager.<name>`. Several files may write to the same name (`base` is assembled by `system/nix` and `system/home-manager`); the module system merges them.
- **A host is declared in the registry.** `modules/flake/hosts` defines `configurations.nixos.<name>` with `system` (one of the flake `systems`, `x86_64-linux` by default), `module` (a NixOS module that imports named modules from `config.flake.modules.nixos`) and `testScript` (lines appended to the boot test). Several files may write to the same host `module` (`default.nix`, `hardware.nix`); they merge. The registry sets `nixpkgs.hostPlatform` itself: never set it in a module. Other `nixpkgs.*` options (overlays, `nixpkgs.config.allowUnfreePackages`) are fine in a host `module`; the boot test runs with `node.pkgsReadOnly = false` so it honors them. A host name may only contain letters, digits, `-` and `_` (it becomes a check name and a CI job name); the registry rejects anything else.
- **A user wires home-manager.** `modules/users/<user>` defines both the NixOS account (`flake.modules.nixos.<user>`) and its home config (`flake.modules.homeManager.<user>`), connected through `home-manager.users.<user>.imports`.
- **Checks are generated, not listed.** Every registry entry gets `checks.<system>.<name>-toplevel` and `checks.<system>.<name>-boots`, a NixOS test that waits for `multi-user.target` and every `home-manager-<user>.service`. Every `x86_64-linux` host without NVIDIA also gets `<name>-nvidia` (`modules/hardware/nvidia/check.nix`): the host extended with the `nvidia` module must evaluate, and its NVIDIA open kernel module must build against the host kernel. A host that imports `nvidia` is already covered by `<name>-toplevel`. A fork with no NVIDIA machine can delete `check.nix`. Likewise every host with `disk` and without `wipe-home` gets `<name>-wipe-home` (`modules/system/wipe-home/check.nix`): the install test of the host extended with `wipe-home`. A host importing `disk` also gets `<name>-disk`: disko's install test, which formats a blank disk, installs, reboots and runs `modules/system/disk/wipe-test.py`, then `modules/system/snapshots/snapshot-test.py` (`mkOrder 1200`); `wipe-home` adds a step before that reboot (`mkOrder 900`) and its checks after it (`mkOrder 1100`), where `disk` otherwise checks `/home` survived; on an `x86_64-linux` host that also imports `secure-boot`, it then boots the installed disk under OVMF with Secure Boot and swtpm and runs `modules/system/secure-boot/secure-boot-test.py`. The boot test runs the host with `disko.enableConfig = false` (plain VM disks); only `<name>-disk` exercises LUKS, btrfs and the rollback. Modules with their own VM test add `profile-<name>`, `system-<name>`, `dev-<name>`, `home-<name>` or `desktop-<name>` through `helpers.profileCheck`, `helpers.baseCheck` (a `base` machine) or `helpers.vmCheck` (its own nodes) (`x86_64-linux` only). `flake.ci.matrix` (in `modules/flake/ci`) lists every check with its system and runner (`ubuntu-24.04`, `ubuntu-24.04-arm`), so new hosts reach CI without touching YAML.

Layers under `modules/`, one folder per tool, never a loose `.nix` file at layer level (`helpers/` excepted):

- `flake/`: flake wiring (systems, formatter, host registry and its checks, dev shell with the git hooks, `sops` and `age`, CI and user scripts: `closure-diff`, `bench`)
- `helpers/`: functions shared by flake-parts modules, declared once in `config.helpers` (`helpers/default.nix`) and added one file per topic; read them with `inherit (config.helpers) <name>;`
- `system/`: OS base shared by every host; `system/disk` is the disko layout (LUKS, btrfs subvolumes) and the initrd rollback of `/`; it assumes UEFI with systemd-boot (ESP on `/boot`, boot entry editor off); `system/impermanence` adds `environment.persistence."/persist"` to `base`, off by default, and `disk` turns it on; `system/resolved` turns systemd-resolved on in `base`; `system/wipe-home` is opt-in, needs `disk`, moves the previous `@home` to `/.snapshots/home-wiped.<date>` in the initrd (kept 7 days, the 2 most recent always; `@home` is created before pruning so a failed delete never blocks the boot) and sets `impermanence.wipeHome`, which drops `home` from btrbk and turns on every home-manager `home.persistence."/persist"` path (off otherwise, through a `sharedModules` default in `system/impermanence`); `system/snapshots` adds btrbk to `disk`; `system/ssh` and `system/secrets` (sops-nix) are opt-in; `nix` (nh, XDG base directories), `nix-index`, `resolved`, `ssh` and `secrets` have a `check.nix` (assertions in `test.py`); `system/secure-boot` is opt-in and swaps systemd-boot for lanzaboote (keys in `/var/lib/sbctl`, enrolled automatically from setup mode); its install-test hook only applies when the host also imports `disk` (`options ? disko`, under `config` to avoid infinite recursion)
- `hardware/`: opt-in modules a host imports for its machine (`firmware`, `audio`, `bluetooth`, `nvidia`, `amd-gpu`, `intel-gpu`); machine-specific profiles come from the `nixos-hardware` input, imported in the host's `hardware.nix`
- `profiles/`: roles a host imports (`desktop`, `laptop` which extends `desktop`, `server`); each has a `check.nix` (assertions in `test.py`) asserting its settings are active; `desktop` carries a `key` so importing it twice (`laptop` plus `desktop`) loads it once; details and the bench in `docs/performance.md`
- `dev/`: opt-in development modules (`docker` rootless, `libvirt`, `nix-ld`, `direnv`); `docker` and `libvirt` have a `check.nix`; details in `docs/dev.md`
- `home/`: home-manager modules a user imports (`zsh`, `starship`, `git`, `neovim`, `cli`) and the `shell` bundle importing them all (with `xdg.enable`); opt-in `stow` links a user's dotfiles, a mirror of the home or stow packages, at activation (`stow-dotfiles.sh`); their values go through `helpers.mkDefaults` (`helpers/defaults.nix`, `mkDefault` on every leaf) so a user overrides any of them without `mkForce`, while lists meant to be extended (`extraPackages`) stay plain; `zsh` also publishes a NixOS `zsh` module (login shell, global `compinit` off so home-manager runs it once); `shell` and `stow` have a `home-<name>` VM test through `helpers.vmCheck`; details in `docs/home.md`
- `users/`, `hosts/`: owned by the fork; upstream keeps them minimal so forks merge upstream without conflicts
- `desktop/`: the graphical layer; `greetd` (tuigreet, option `greetd.command`) and `plymouth` are NixOS modules a host imports; `theme` publishes both a NixOS module (fonts, dconf) and a home-manager one (`theme.palette`, `theme.font`, GTK, Qt, cursor) that every desktop module reads; `waybar`, `fuzzel`, `ghostty`, `mako` and `lock` (hyprlock, hypridle, plus its NixOS PAM entry) are home-manager modules; `hyprland` (NixOS `default.nix`, home `home.nix`) imports the rest and hands its home module to every user through `home-manager.sharedModules`; values go through `helpers.mkDefaults`; `hyprland` and `plymouth` have a `desktop-<name>` VM test; details in `docs/desktop.md`

Starter-owned modules (`flake/`, `system/`, future layers) stay generic: no user name, host name, path or personal value in them. Personal values belong in `hosts/` and `users/`, or behind an option with a neutral default.

## Pitfalls already hit

- **`let` shadows `with`.** In `with config.flake.modules.nixos; [ desktop ]`, a `let desktop = ...` in scope silently wins and imports the wrong value. Never bind a name that matches a module name.
- **`nix flake check` builds the boot test**; a passing `nix eval` does not mean the system boots.
- **`/` is wiped at every boot.** State a module writes outside `/nix`, `/home`, `/persist` and `/.snapshots` disappears unless it is added to `environment.persistence."/persist"`. Declare it in the module that owns the state (`profiles/desktop` keeps NetworkManager's), never in a central list; the option exists on every host, so this evaluates even without `disk`. State in a home goes through home-manager's `home.persistence."/persist".directories`, relative to the home (`helpers.relativeToHome`), from the home module that owns it, or from a NixOS module through `helpers.persistInEveryHome` (`docker`, `direnv`, `audio`, `hyprland`, `wipe-home`); it only applies on hosts with `wipe-home`. Passwords come from `/persist/passwords/<user>` (`users.mutableUsers = false`).
- **`home-manager-<user>` fails inside `<name>-disk`.** disko's harness mounts the host store over 9p and `nix-daemon` cannot chown it; the wipe test does not wait for it, and `<name>-boots` covers home-manager.
- **Plain systemd-boot is not tested in CI.** `example` imports `secure-boot`, so `<name>-disk` only covers the lanzaboote path. A change to `disk` that could break a host without `secure-boot` is checked by evaluating such a host by hand.
- **State read during activation lives under `config.impermanence.persistPrefix`.** Activation scripts (sops-nix decrypting secrets) run before the impermanence bind mounts exist: point such paths straight at `/persist` through the prefix, as `ssh` and `secrets` do, instead of listing them in `environment.persistence`.
- **No key file next to `tpm2-device`.** With `tpm2-device=auto`, systemd-cryptsetup reads a configured key file as a TPM token blob and never tries the TPM. Tests unlock LUKS by typing the passphrase on the console (`modules/system/disk/unlock-disk.py`), never through a key file or `boot.initrd.secrets`.
- **Wait for the password agent, not for its unit.** `Started ... Password Requests to Console` also matches the earlier `Directory Watch` unit; keys sent then are lost. Wait for `password query on /dev/tty1`, where QEMU's keyboard types, or, with Plymouth, for `Please enter passphrase for disk` (`unlock-disk.py` accepts both).
- **Hyprland needs a DRM device in the VM.** QEMU's default VGA gives none: VM tests and `system.build.vm` of a Hyprland host use `-vga none -device virtio-gpu-pci`.
- `warning: unknown flake output 'modules'` is expected (flake-parts `modules` output) and harmless.

## CI

Workflows in `.github/workflows/` only chain one-line calls. Any logic goes into Nix (`flake.ci.*`) or into a shell script under `modules/flake/ci/`, packaged with `writeShellApplication` (shellcheck at build time, pinned dependencies) and called with `nix run .#<name>`. Never write a multi-line script inside a workflow. The `ci-ok` job is the single required status check on `main`. Details: `docs/ci.md`.

## Conventions

- **Unfree packages go through `nixpkgs.config.allowUnfreePackages`.** A module that needs one adds its name (`lib.getName`) to that list; nixpkgs concatenates the lists of every module and still honors a host's `allowUnfreePredicate`. `allowUnfree = true` accepts every unfree package and makes the lists moot.
- **No comments** in `.nix` and `.sh` files: names carry the intent. Tool directives stay.
- Code, docs, commits and README in English. No em dash or en dash anywhere.
- Conventional Commits (`feat`, `fix`, `refactor`, `docs`, `chore`, `ci`), subject describing the observable effect, lowercase, no trailing period, at most 72 characters; prose body explaining why when not trivial. release-please reads them on `main` to open the release PR (version, `CHANGELOG.md`): never set a version, edit `.release-please-manifest.json`, `version.txt` or `CHANGELOG.md`, or create a tag by hand. The project stays in `0.x` (`bump-minor-pre-major` in `release-please-config.json`) until the maintainer decides on `1.0.0`.
- Branches `feat/`, `fix/`, `refactor/`, `chore/`, `docs/` from `develop`; PR into `develop`, then `develop` into `main`. `main` only moves through PRs. A `develop` into `main` PR is merged with a merge commit, never squashed: release-please must see every Conventional Commit.
- `ROADMAP.md` lists shipped lots and their guardrail; update it in the commit that ships a lot.
- The pre-push hook (git-hooks.nix, `modules/flake/git-hooks`) runs treefmt, `nix flake check --no-build` and actionlint, in seconds; builds and boot tests are left to CI on purpose. Never bypass it with `--no-verify`: a failing hook means a cause to fix.
- Stage files one by one. Never commit or push without the maintainer's explicit go: show the staged diff and wait.

## Safety

Everything is developed and tested in VMs. Never run disko, `nixos-install`, `nixos-rebuild switch`, `bootctl`, `sbctl` or any partitioning command against a physical disk or the host machine without an explicit request naming the target.

## Done means

1. `nix fmt` leaves nothing to change and `nix flake check -L` ends with `all checks passed!`
2. `nix run nixpkgs#actionlint` is clean when workflows changed
3. A change with an observable effect is shown running: VM boot, `closure-diff`, command output
4. README and `docs/ci.md` still match the code
