# Releases of nixos-forge

How upstream (`axelhamil/nixos-forge`) publishes a release. A fork runs none of this: its `release` workflow is skipped, and it takes releases with `nix run .#upstream` ([updates.md](../explanation/updates.md#your-fork-follows-upstream)).

## Branches

- Work lands on `develop` through PRs from `feat/`, `fix/`, `refactor/`, `chore/` and `docs/` branches.
- `main` moves only through PRs, the release and the weekly lock PR.
- `sync-develop` merges `main` back into `develop` after every push to `main`, with the App token, so `check` runs on the merged `develop`.

## The release flow

The `release` workflow runs on every push to `develop`, only when `github.repository` is `axelhamil/nixos-forge`:

1. [release-please](https://github.com/googleapis/release-please) keeps a release PR against `develop` that bumps the version and `CHANGELOG.md` from the Conventional Commits. Its checks are the last proof before a release.
2. Merging it tags the release and publishes the GitHub Release. release-please writes the release notes alone.
3. The workflow then moves `main` to the release with the App token.

Never set a version, edit `.release-please-manifest.json`, `version.txt` or `CHANGELOG.md`, or create a tag by hand. The project stays in `0.x`, every release marked as a pre-release, until the maintainer decides on `1.0.0`.

## What each commit releases

| Commit | While in `0.x` |
|---|---|
| breaking (`!` or a `BREAKING CHANGE:` footer, which must say how to migrate) | bumps the minor |
| `feat`, `fix` | bump the patch |
| `chore`, `ci`, `docs`, `refactor` | no release of their own |

Release notes open with the breaking changes, then features, fixes, performance, documentation and reverts. `chore`, `ci`, `refactor` and `test` stay out of them (`release-please-config.json`).

The release PR footer (`release-please-config.json`) points forks at `nix run .#upstream`.

## Repository settings

On top of what a fork sets ([ci.md](../how-to/ci.md#turn-ci-on-in-your-fork)), the ruleset on `main` lists the bot App in its bypass list, so `release` can move `main` to a release.

## Where forks find upstream

`nix run .#upstream` falls back to `https://github.com/axelhamil/nixos-forge.git` when it gets no argument and the fork has no `upstream` remote, and the `sync-upstream` workflow to `axelhamil/nixos-forge` when the variable `UPSTREAM` is not set. `sync-upstream` never runs upstream itself.
