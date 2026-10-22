# Contributing

Thanks for helping improve nixos-forge, the forge where you build your own NixOS distro.

- Architecture, conventions and pitfalls are in [AGENTS.md](AGENTS.md); read it before a non-trivial change.
- Step-by-step recipes to add an app, feature, hardware module, profile, host or input are in the [`extend-flake` skill](.agents/skills/extend-flake/SKILL.md).
- To try nixos-forge first, follow the [README](README.md).

## Setup

Fork the repository on GitHub, clone your fork and run `nix develop` once. It installs a pre-push hook (treefmt, `nix flake check --no-build`, actionlint) that runs in seconds, and gives the Rust toolchain of the flake. You need Nix with flakes, and KVM to run the boot tests.

## Before a PR

| Command | When |
|---|---|
| `nix fmt` | always |
| `nix flake check -L` | always: lint, host builds, boot tests |
| `nix run .#forge -- check-docs` | after touching `README.md` or `docs/`: links, anchors, the `docs/README.md` index, a section per module; the `docs` check runs the same |
| `cargo clippy --all-targets`, `cargo nextest run`, in `crates/` from `nix develop` | after touching `crates/`; a changed report rewrites its snapshot with `INSTA_UPDATE=always cargo nextest run`: review the `.snap` diff. The `rust-*` checks run the same |
| `nix run .#options-doc` | after touching `modules/flake/hosts/options.nix`, to regenerate `docs/reference/options.md` |
| `nix run nixpkgs#actionlint` | after touching `.github/workflows/` |

New files must be `git add`ed before any `nix` command, or the flake does not see them.

A new or renamed profile, feature, hardware module, app or registry field needs a line, enabled or commented out, in each of the three example hosts under `modules/hosts/`. A new person field needs one in `modules/users/alice`. The `example-hosts` check fails on a missing line and names it.

`modules/flake/docs/pending.txt` lists known docs violations waiting for a fix. It may only shrink: fix a violation rather than adding it, and delete the line of one you fixed.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): they drive releases through release-please. A breaking change (`!` or a `BREAKING CHANGE:` footer) must say how to migrate. Never set a version, edit `CHANGELOG.md` or create a tag by hand. What each type releases is in [docs/maintenance/releases.md](docs/maintenance/releases.md#what-each-commit-releases).

```text
feat: add a server profile
fix: keep the boot test working without home-manager users
```

## Pull requests

1. Branch from `develop` (`feat/`, `fix/`, `refactor/`, `chore/`, `docs/` plus a slug).
2. Run the commands above.
3. Open the PR from your fork against `develop`. CI builds and boots every host and posts the package diff.
