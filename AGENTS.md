# AGENTS.md

**This file is the source of truth for agent instructions.** Claude Code (2.1.277 and later), Cursor, Codex and other AGENTS.md tools read it. Add agent guidance here only; never add a `CLAUDE.md`, which would hide this file from Claude Code. Step-by-step recipes (add an app, feature, hardware module, profile, host, person, helper, CI script or input) live in the `extend-flake` skill, `.agents/skills/extend-flake/SKILL.md`, linked from `.claude/skills/<name>` since Claude Code reads skills there only.

## What this is

nixos-forge is not a distro: it is the forge where you build your own NixOS distro. Forks rename the example user and host, then grow their own system; CI, its headline strength, tests every machine (build, VM boot, install on a blank disk). The original repository, seen from a fork, is upstream. Desktops and Raspberry Pi hosts are both in scope. User-facing behavior is in `README.md` and `docs/`; this file holds the rules an agent needs to change the code safely.

## The forge rule

**nixos-forge ships mechanisms, forks choose policy.** Before adding or changing any setting, ask whether two forks could choose differently without either being wrong. If not, it is a mechanism and belongs to the forge (the registry, its checks, impermanence, `forge doctor`, `forge update`). If so, it is policy: a named profile, app or feature a host lists, or at most a `mkDefault` a fork overrides without `mkForce`. `base` is imposed on every fork, so it holds mechanisms only. A fork that ran `nix run .#init` is a distro and sets whatever policy it wants.

## Commands

| Goal | Command |
|---|---|
| Make a fresh fork yours (`--help` for the flags) | `nix run .#init` |
| Pre-push hook (once per clone), `sops`, `age`, the Rust toolchain | `nix develop` |
| Run the pre-push hook by hand | `nix develop -c pre-commit run --hook-stage pre-push --all-files` |
| Format and lint (nixfmt, statix, deadnix) | `nix fmt` |
| All checks: lint, host builds, boot tests | `nix flake check -L` |
| One check | `nix build -L .#checks.x86_64-linux.<name>` |
| Checks CI runs, with system and runner | `nix eval --json .#ci.matrix` |
| Build the VM, boot it headless | `nix build .#nixosConfigurations.<host>.config.system.build.vm`, then `QEMU_KERNEL_PARAMS=console=ttyS0 ./result/bin/run-<host>-vm -nographic` |
| Build the installer ISO of a host with `disk` | `nix build .#installer-<host>` |
| Full install in a local VM (Secure Boot setup mode, TPM2; `--reset`) | `nix run .#try-install -- <host>` |
| Regenerate `docs/reference/options.md` after changing the registry options | `nix run .#options-doc` |
| Check the docs (links, anchors, index, names a fork lacks, a section per module) | `nix run .#forge -- check-docs` |
| Lint and test Rust, from `nix develop` in `crates/` (`INSTA_UPDATE=always` rewrites snapshots: review the `.snap` diff) | `cargo clippy --all-targets && cargo nextest run` |
| Package diff against a revision | `nix run .#closure-diff -- <rev>` |
| Show what upstream changed and prepare its merge in a fork | `nix run .#upstream` |
| Evaluate an `aarch64-linux` check | `nix eval --raw .#checks.aarch64-linux.<name>.drvPath` |
| Lint workflows | `nix run nixpkgs#actionlint` |

A flake only sees files tracked by git: `git add` new files (staging is enough) before any `nix` command, or they are silently missing.

## Architecture

The dendritic pattern: `flake.nix` hands `modules/` to `import-tree`, and every `.nix` file under it is a flake-parts module (a path segment starting with `_` is ignored). Details: `docs/explanation/architecture.md`.

- **Publish, never import.** A folder publishes NixOS config in the namespace of its place (`base`, `profiles`, `features`, `hardware`, `apps`). Each registry field resolves in its own namespace only (`apps` falls back to `pkgs.<name>`, `hardware` to a nixos-hardware profile); a name published by two namespaces throws (`flake-modules`); an app has a `nixos` and a `homeManager` part, and a home-manager module in the `nixos` part throws. `flake.modules.*` are outputs for other flakes only: nothing in the flake reads or writes them.
- **A host is a registry entry in plain values** (`modules/flake/hosts/options.nix`, fields in `docs/reference/options.md`). Names are lists, never `enable` booleans, resolved at the flake-parts level in the registry's `config.module`: never translate them from inside a NixOS module.
- **No app imports another app or a feature.** An app keeps only what it cannot start without; every choice (terminal, launcher, bar, prompt, editor...) is an app or feature the host or person lists. An app fits its neighbours by reading their home-manager options. Tools with nothing to configure are nixpkgs names, not apps.
- **The registry sets `nixpkgs.hostPlatform`**: never set it in a module. **No registry field names a device**: the disk is chosen at install, the system mounts by partlabel.
- **A person is defined once** in `modules/users/<user>`; a host's `users.<user>` holds per-machine fields only.
- **Checks are generated, not listed** (`docs/reference/ci.md`). Test machines pin `configurationRevision` with `helpers.testRevision`; module tests use `helpers.<namespace>Check` or `vmCheck` (`x86_64-linux` only; `appCheck` uses a `tester` user).
- **Time budgets.** Every VM test is bounded by its `globalTimeout`, about 3 times its longest green run in CI (`vmCheck` and its wrappers, a direct `runNixOSTest`, a disko `installTest` through `helpers.diskTest`); the derivation and job limits sit above it. A wait that can hang on something outside the test script (console text, a password prompt, the firmware, a long `nix copy` or install) gets an explicit `timeout=` below that budget; no test waits for a fixed delay.
- **VM launchers.** No test machine has a network card (`-nic none` through `machine_command`). A test that starts its own machine uses `create_uefi_machine` or `create_secure_machine`; any other `create_*machine` call must pass `start_command=machine_command(...)`, and no other hand-built QEMU command is allowed (only NixOS run scripts, listed in `modules/flake/helpers/vm-launchers-check.nix`). disko install tests go through `helpers.diskTest`. The `vm-launchers` check names each offender. Details: `docs/reference/ci.md#test-machines`.
- **Helpers** live in `config.helpers` (`modules/flake/helpers/`, one file per topic), read with `inherit (config.helpers) <name>;`. App values go through `helpers.mkDefaults`; lists meant to be extended stay plain.
- **Layout.** One folder per tool under `modules/{flake,features,apps,users,hosts}`, never a loose `.nix` file at layer level (`flake/helpers/` excepted). `users/` and `hosts/` belong to the fork; upstream keeps them minimal (the person `alice`, the hosts `example`, `example-server`, `example-pi`).
- **`forge init` rewrites the examples by line** (`crates/forge/src/init/`): the `"name" # description` lines of `features`, `hardware`, `apps`; the plain host fields (`system`, `profile`, `flake`, `timeZone`, `locale`, `keyboard`), the person fields (`fullName`, `email`), the `users.alice = {` line, the `flake = "github:` and `"auto-upgrade"` lines. Keep that layout. The `init` check runs it on copies of the flake (`modules/flake/init/_call-fork.nix`, locked inputs, no fetch) and evaluates every remaining host's `toplevel.drvPath`.
- **Rust**: `crates/` holds `forge` (every host), `forge-ci` (CI scripts, no host) and `forge-apps` (hosts listing its app); `forge` commands follow `docs/reference/forge.md`.

Modules shipped by nixos-forge stay generic: no user name, host name, path or personal value in them. Personal values belong in `hosts/` and `users/`, or behind an option with a neutral default.

## Pitfalls already hit

- **`let` shadows `with`.** In `with config.profiles; [ desktop ]`, a `let desktop = ...` in scope silently wins. Never bind a name that matches a module name.
- **`nix flake check` builds the boot test**; a passing `nix eval` does not mean the system boots.
- **`/` is wiped at every boot.** State outside `/nix`, `/home`, `/persist` and `/.snapshots` goes in `environment.persistence."/persist"`, declared by the module that owns it, never in a central list; state meant to vanish in `doctor.ephemeralPaths`.
- **Home state** goes through `home.persistence."/persist".directories` (relative to the home, `helpers.relativeToHome`), or `helpers.persistInEveryHome` from a NixOS module; it only applies on hosts with `wipe-home`.
- **State read during activation lives under `config.impermanence.persistPrefix`.** Activation scripts (sops-nix) run before the bind mounts exist, as `ssh` and `secrets` show.
- **`home-manager-<user>` fails inside `<name>-disk`** (store shared over 9p); `<name>-boots` covers it.
- **No key file next to `tpm2-device`**: systemd-cryptsetup then reads it as a TPM token. Tests type the passphrase on the console (`modules/features/disk/unlock-disk.py`).
- **The volume key of an open LUKS2 device is not readable** through dm-crypt. `forge enroll tpm2` uses the `user` copy the initrd links with `link-volume-key=` (`%user:<name>`); the initrd drops it once a token exists, else after 300 s.
- **Wait for the password agent, not its unit**: `password query on /dev/tty1` (Plymouth: `Please enter passphrase for disk`); earlier keys are lost.
- **No `bootindex` on test disks**: `disks()` sets none, since OVMF then rewrites its boot order and deletes entries of devices without one; only the installer test passes `disk_drive(..., boot_first=True)`.
- **A TPM test machine needs a network backend**: without the `-netdev user` of `create_secure_machine`, OVMF stalls and a TPM2 token enrolled on that boot never unlocks the next.
- **Hyprland needs a DRM device in the VM**: `-vga none -device virtio-gpu-pci`.
- **nixos-raspberrypi's kernels are rebuilt from our `pkgs`** (`mkOverride 900`): its cache has no hit.
- **`nix flake check` only evaluates the current system.** Evaluate `aarch64-linux` checks with `nix eval` and let CI build them.
- **Unfree packages go through `nixpkgs.config.allowUnfreePackages`**: a module adds its `lib.getName`; never `allowUnfree = true`. The boot test runs with `node.pkgsReadOnly = false` so host `nixpkgs.*` settings apply.
- `warning: unknown flake output 'modules'` is expected and harmless.

## CI

- Workflows in `.github/workflows/` only chain one-line calls. Logic goes into Nix (`flake.ci.*`), into `forge-ci` (`crates/forge-ci`) behind a wrapper package of `modules/flake/ci/`, or, under about 20 lines, into a `writeShellApplication` script under `modules/flake/ci/` (or its tool's folder) run with `nix run .#<name>`. Never a multi-line script inside a workflow.
- `ci-ok` is the single required status check on `main`. `release` runs upstream only, `sync-upstream` in forks only, both keyed on `github.repository`.
- Every text the CI writes for people is static, in the program that writes it (`crates/forge-ci/src/report/sections.rs`, `crates/forge/src/upstream/notes.rs`); release-please writes the release notes alone.
- `modules/flake/docs/pending.txt` only shrinks: fix a new docs violation, never list it.
- Details: `docs/reference/ci.md` (workflows, checks, test machines, scripts) and `docs/how-to/ci.md` (fork settings, report).

## Conventions

- **No comments** in `.nix`, `.sh` and `.rs` files: names carry the intent. Rust uses guard clauses, typed errors (`thiserror`) and no `unwrap` or `expect` outside tests. Tool directives stay.
- **Except the examples** (`example`, `example-server`, `example-pi`, `modules/users/alice`), commented for a forker. Each example host has one or two lines per registry field and one line per shipped profile, feature, hardware module and app, enabled or commented out; the example person one or two lines per person field.
- **A change that adds or renames a registry field, a person field, a profile, a feature, a hardware module or an app updates all three example hosts (or the example person) in the same commit**, and `docs/reference/options.md` with `nix run .#options-doc`. A commented line never changes what the host builds. The `example-hosts` check names a missing line. This binds upstream only: a fork after `init` has no example host left.
- Code, docs, commits and README in English. No em dash or en dash anywhere.
- Conventional Commits (`feat`, `fix`, `refactor`, `docs`, `chore`, `ci`), subject describing the observable effect, lowercase, no trailing period, at most 72 characters; prose body explaining why when not trivial.
- release-please opens the release PR on `develop`: never set a version, edit `.release-please-manifest.json`, `version.txt` or `CHANGELOG.md`, or create a tag by hand. The project stays in `0.x`, every release a pre-release, until the maintainer decides on `1.0.0`.
- Branches `feat/`, `fix/`, `refactor/`, `chore/`, `docs/` from `develop`; PR into `develop`. The merged release PR tags the release and `release` moves `main` to it; otherwise `main` moves only through PRs (the weekly lock PR, merged back by `sync-develop`).
- `ROADMAP.md` lists shipped lots and their guardrail checks, then the planned lots in order; the next lot is the first planned one. The commit that ships a lot moves its row to Shipped with the checks that actually guard it.
- The pre-push hook runs treefmt, `nix flake check --no-build` and actionlint in seconds; builds and boot tests are left to CI. Never bypass it with `--no-verify`.
- Stage files one by one. Never commit or push without the maintainer's explicit go: show the staged diff and wait.

## Safety

Everything is developed and tested in VMs. Never run disko, `nixos-install`, `nixos-rebuild switch`, `bootctl`, `sbctl` or any partitioning command against a physical disk or the host machine without an explicit request naming the target.

## Done means

1. `nix fmt` leaves nothing to change and `nix flake check -L` ends with `all checks passed!`
2. `nix run nixpkgs#actionlint` is clean when workflows changed
3. A change with an observable effect is shown running: VM boot, `closure-diff`, command output
4. README and `docs/` still match the code
