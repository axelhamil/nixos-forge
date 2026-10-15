# AGENTS.md

**This file is the source of truth for agent instructions.** Claude Code (2.1.277 and later), Cursor, Codex and other tools that follow the AGENTS.md convention read it directly. Add agent guidance here and nowhere else, and do not add a `CLAUDE.md`: Claude Code reads `AGENTS.md` only when no `CLAUDE.md` exists.

## What this is

A NixOS flake meant to be forked (the kickstart.nvim model): a readable starting point, not a framework. Forks rename the example user and host, then grow their own system. Desktops and Raspberry Pi hosts are both in scope.

## Commands

| Goal | Command |
|---|---|
| Install the pre-push hook (once per clone) | `nix develop` |
| Run the pre-push hook by hand | `nix develop -c pre-commit run --hook-stage pre-push --all-files` |
| Format and lint (nixfmt, statix, deadnix) | `nix fmt` |
| All checks: lint, host builds, boot tests | `nix flake check -L` |
| One check | `nix build -L .#checks.x86_64-linux.<name>` |
| Checks the CI matrix runs | `nix eval --json .#ci.checks` |
| Build the VM | `nix build .#nixosConfigurations.<host>.config.system.build.vm` |
| Boot the VM headless (blocks until killed) | `QEMU_KERNEL_PARAMS=console=ttyS0 ./result/bin/run-<host>-vm -nographic` |
| Package diff against a revision | `nix run .#closure-diff -- <rev>` |
| Lint workflows | `nix run nixpkgs#actionlint` |

A flake only sees files tracked by git: `git add` new files (staging is enough) before any `nix` command, or they are silently missing.

## Architecture: the dendritic pattern

The flake is built with flake-parts, and `flake.nix` hands the whole `modules/` tree to `import-tree`. Consequences:

- **Every `.nix` file under `modules/` is a flake-parts module**, imported automatically. There is no import list: adding a file adds the feature, deleting it removes it. A path segment starting with `_` is ignored by import-tree.
- **NixOS and home-manager config is published, not imported.** A file contributes to `flake.modules.nixos.<name>` or `flake.modules.homeManager.<name>`. Several files may write to the same name (`base` is assembled by `system/nix`, `system/locale`, `system/home-manager`); the module system merges them.
- **A host assembles named modules.** `modules/hosts/<host>/default.nix` defines `flake.nixosConfigurations.<host>` from `config.flake.modules.nixos` and sets `nixpkgs.hostPlatform` in that list, not inside a shared module: NixOS tests (`runNixOSTest`) impose their own read-only `pkgs` and fail on any `nixpkgs.*` option in the modules they import.
- **A user wires home-manager.** `modules/users/<user>` defines both the NixOS account (`flake.modules.nixos.<user>`) and its home config (`flake.modules.homeManager.<user>`), connected through `home-manager.users.<user>.imports`.
- **Checks are derived, not listed.** `modules/flake/checks` creates `<host>-toplevel` for every `nixosConfigurations` entry of the current system; boot tests live next to their host (`modules/hosts/<host>/test.nix`). `flake.ci.checks` (in `modules/flake/ci`) feeds the CI matrix, so new checks reach CI without touching YAML.

Layers under `modules/`, one folder per tool, never a loose `.nix` file at layer level:

- `flake/`: flake wiring (systems, formatter, checks, CI scripts)
- `system/`: OS base shared by every host
- `users/`, `hosts/`: owned by the fork; upstream keeps them minimal so forks merge upstream without conflicts
- `desktop/`, `apps/`: planned

Starter-owned modules (`flake/`, `system/`, future layers) stay generic: no user name, host name, path or personal value in them. Personal values belong in `hosts/` and `users/`, or behind an option with a neutral default.

## Pitfalls already hit

- **`let` shadows `with`.** In `with config.flake.modules.nixos; [ desktop ]`, a `let desktop = ...` in scope silently wins and imports the wrong value. Never bind a name that matches a module name.
- **`nix flake check` builds the boot test**; a passing `nix eval` does not mean the system boots.
- `warning: unknown flake output 'modules'` is expected (flake-parts `modules` output) and harmless.

## CI

Workflows in `.github/workflows/` only chain one-line calls. Any logic goes into Nix (`flake.ci.*`) or into a shell script under `modules/flake/ci/`, packaged with `writeShellApplication` (shellcheck at build time, pinned dependencies) and called with `nix run .#<name>`. Never write a multi-line script inside a workflow. The `ci-ok` job is the single required status check on `main`. Details: `docs/ci.md`.

## Conventions

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
