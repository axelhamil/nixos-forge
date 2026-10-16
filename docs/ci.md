# CI and repository settings

How the GitHub Actions workflows work, and what to configure on your fork so they run.

## Workflows

| Workflow | Trigger | What it does |
|---|---|---|
| `check` | every PR, push to `main` and `develop` | Lints the flake and checks `flake.lock` health, then runs one job per flake check (host builds, VM boot tests, the SD card image instead of the host build and boot test for Raspberry Pi hosts, and for hosts importing `disk` an install test on a blank disk that reboots and checks `/` is wiped and that a btrbk snapshot lands in `@snapshots`, and, with `wipe-home` added on hosts that lack it (`<host>-wipe-home`), that `/home` starts empty while declared paths survive, then for `x86_64-linux` hosts importing `secure-boot` boots it under OVMF with an emulated TPM and checks the disk unlocks with the TPM alone; each module with a `check.nix` adds a `profile-<name>`, `system-<name>`, `dev-<name>`, `home-<name>` or `desktop-<name>` VM test; every `x86_64-linux` host without NVIDIA also evaluates with the `nvidia` module and builds the NVIDIA open kernel module against its kernel), each capped at 60 minutes (240 for an SD card image, whose vendor kernel compiles for more than an hour without a binary cache) on a runner of the host's system: `ubuntu-24.04` with KVM, or `ubuntu-24.04-arm`. The `ci-ok` job sums up the result. |
| `diff` | every PR | Builds each host on the PR and on its base, one job per system on a runner of that system (`ubuntu-24.04`, `ubuntu-24.04-arm`), then posts the package diff ([nvd](https://git.sr.ht/~khumba/nvd)) as a PR comment and in the job summary. |
| `update-flake-lock` | Monday 06:00 UTC, or by hand | Opens a PR that updates `flake.lock` against `main` and enables auto-merge on it. |
| `sync-develop` | push to `main` | Merges `main` into `develop`. |
| `release` | push to `main` | Runs [release-please](https://github.com/googleapis/release-please): keeps a release PR open that bumps the version and updates `CHANGELOG.md` from Conventional Commits. Merging that PR tags the version and publishes the GitHub Release. Weekly lock updates (`chore`) do not release. |
| Dependabot | weekly | Updates the GitHub Actions versions, with PRs against `develop`. |

The repo uses two long-lived branches: work lands on `develop`, then reaches `main` through a PR. When you fork, keep the `develop` branch (untick "Copy the main branch only"), or create it from `main`.

## Settings on your fork

1. **Enable the workflows**: GitHub disables them on forks. Open the Actions tab and enable them.
2. **Bot GitHub App**: the release PR and the weekly lock PR are opened with a GitHub App token, because PRs opened with the default `GITHUB_TOKEN` do not trigger workflows, so CI and auto-merge would never run on them. [Create a GitHub App](https://github.com/settings/apps/new) on your account with webhooks off and the repository permissions **Contents** and **Pull requests** set to read and write, generate a private key, and install the App on your fork. Then set the repository variable `BOT_CLIENT_ID` to its client ID and the secret `BOT_PRIVATE_KEY` to the key. One App serves all your repositories.
3. **Allow auto-merge**: Settings, General, "Allow auto-merge".
4. **Ruleset on `main`**: require a pull request and require the `ci-ok` status check. Without a required check, auto-merge has nothing to wait for and merges at once.
5. **Ruleset on `develop`**: block deletions and force pushes. Without it, "Automatically delete head branches" deletes `develop` as soon as a `develop` into `main` PR merges.
6. **Binary cache (optional)**: set the repository variable `CACHIX_CACHE` to your [Cachix](https://www.cachix.org) cache name and the secret `CACHIX_AUTH_TOKEN` to its token. The `check` and `diff` workflows then pull from and push to it, leaving out what cache.nixos.org already serves (Cachix's upstream cache) and large files that are cheap to rebuild from cached parts: sources, SD card and disk images, VMs. What is left is mostly kernels and test results, sized for the free plan's 5 GB; when full, Cachix drops the least recently used paths. Without them, those steps are skipped. Jobs that compile a kernel (an SD card image, the `aarch64-linux` package diff) also keep their Nix store in the GitHub Actions cache with [cache-nix-action](https://github.com/nix-community/cache-nix-action), no account needed: pushes to `main` and `develop` save it (a PR's cache is readable by that PR only), keyed by `flake.lock` and `modules/hardware`, older entries purged; PRs restore it. A PR never compiles a kernel: when no cache matches its `flake.lock` and `modules/hardware` (a lock update, a hardware change), it only evaluates the SD card image and the package diff says it was not built, and the push to `develop` after the merge builds and caches it. `cache` in `nix eval --json .#ci.matrix` says which jobs do.

Scheduled workflows and Dependabot only run from the default branch: the weekly update and Dependabot start once the workflows are on `main`.

## Running the CI scripts locally

The CI matrix and scripts are defined in Nix (`modules/flake/ci/`), so they run the same way on your machine.

List the checks the `build` job runs, with their system and runner (everything except `treefmt`, which the `lint` job covers):

```sh
nix eval --json .#ci.matrix
```

One entry per check:

```json
{"cache":false,"check":"example-boots","runner":"ubuntu-24.04","system":"x86_64-linux","timeout":60}
```

Build one of them as CI does:

```sh
nix build -L .#checks.x86_64-linux.example-boots
```

Print the package diff between a base revision and the current tree:

```sh
nix run .#closure-diff -- <base rev or branch>
```

It diffs the hosts listed for the current platform in `nix eval --json .#ci.hosts`.

Merge `main` into `develop` and push, as the `sync-develop` workflow does (run it from a checkout of `develop`):

```sh
nix run .#sync-develop
```
