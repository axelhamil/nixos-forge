# Contributing

Thanks for helping improve the starter. Architecture, conventions and pitfalls are in [AGENTS.md](AGENTS.md); read it before a non-trivial change. Step-by-step recipes to add an app, feature, hardware module, profile, host or input are in the [`extend-flake` skill](.agents/skills/extend-flake/SKILL.md). To try the starter first, follow the [README](README.md).

## Setup

Fork the repository on GitHub, clone your fork and run `nix develop` once: it installs a pre-push hook (treefmt, `nix flake check --no-build`, actionlint) that runs in seconds. You need Nix with flakes, and KVM to run the boot tests.

## Before a PR

| Command | When |
|---|---|
| `nix fmt` | always |
| `nix flake check -L` | always: lint, host builds, boot tests |
| `nix run .#check-docs` | after touching `README.md` or `docs/`: links, anchors, the `docs/README.md` index, a section per module; the `docs` check runs the same |
| `nix run .#options-doc` | after touching `modules/flake/hosts/options.nix`, to regenerate `docs/reference/options.md` |
| `nix run nixpkgs#actionlint` | after touching `.github/workflows/` |

New files must be `git add`ed before any `nix` command, or the flake does not see them.

A new or renamed profile, feature, hardware module, app or registry field needs a line, enabled or commented out, in each of the three example hosts under `modules/hosts/` (a new person field: in `modules/users/alice`). The `example-hosts` check fails on a missing line and names it.

`modules/flake/docs/pending.txt` lists known docs violations waiting for a fix. It may only shrink: fix a violation rather than adding it, and delete the line of one you fixed.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/) drive releases through release-please. While in `0.x`, a breaking change (`!` or a `BREAKING CHANGE:` footer, which must say how to migrate) bumps the minor, `feat` and `fix` bump the patch, and `chore`, `ci`, `docs` and `refactor` do not release. Release notes open with the breaking changes, then features, fixes, performance, documentation and reverts; `chore`, `ci`, `refactor` and `test` stay out of them. Never set a version, edit `CHANGELOG.md` or create a tag by hand.

```text
feat: add a server profile
fix: keep the boot test working without home-manager users
```

## Pull requests

1. Branch from `develop` (`feat/`, `fix/`, `refactor/`, `chore/`, `docs/` plus a slug).
2. Run the commands above.
3. Open the PR from your fork against `develop`. CI builds and boots every host and posts the package diff.
