# CI and repository settings

What the GitHub Actions workflows do, and what a fork sets so they run.

## Turn CI on in your fork

1. **Keep `develop`**: untick "Copy the main branch only" when forking, or create `develop` from `main`. Work lands on `develop`, then reaches `main` through a PR.
2. **Enable the workflows**: GitHub disables them on forks. Open the Actions tab and enable them.
3. **Bot GitHub App**: the release PR and the weekly lock PR need an App token, since PRs opened with `GITHUB_TOKEN` trigger no workflow. [Create a GitHub App](https://github.com/settings/apps/new) with webhooks off and the repository permissions **Contents** and **Pull requests** set to read and write, generate a private key, install the App on your fork, then set the repository variable `BOT_CLIENT_ID` to its client ID and the secret `BOT_PRIVATE_KEY` to the key. One App serves all your repositories.
4. **Allow auto-merge**: Settings, General, "Allow auto-merge".
5. **Ruleset on `main`**: require a pull request and the `ci-ok` status check. Without a required check, auto-merge merges at once.
6. **Ruleset on `develop`**: block deletions and force pushes, or "Automatically delete head branches" deletes `develop` when a `develop` into `main` PR merges.
7. **Binary cache (recommended)**: create a public [Cachix](https://www.cachix.org) cache, set the repository variable `CACHIX_CACHE` to its name and the secret `CACHIX_AUTH_TOKEN` to a read and write token. Every job pulls from it; only pushes to `main` and `develop` write what they built, except sources and images (SD card, ISO, disk, VM). A PR reads the cache but never gets the token, so code under review, such as an upstream sync PR, cannot poison it; the push after the merge builds and uploads what the PR built. Without it, every run builds every check and PRs never build kernels.

Scheduled workflows and Dependabot only run from the default branch, so they start once the workflows are on `main`.

## Workflows

| Workflow | Trigger | What it does |
|---|---|---|
| `check` | every PR, push to `main` and `develop` | Lints the flake, plans the build with `ci-plan` (checks already in the binary cache are skipped), builds each remaining check on a runner of its system (`ubuntu-24.04` with KVM, or `ubuntu-24.04-arm`), and on a PR posts the package diff ([nvd](https://git.sr.ht/~khumba/nvd)) per system. `ci-ok` sums up the result. |
| `update-flake-lock` | Monday 06:00 UTC, or by hand | Opens a PR updating `flake.lock` against `main`, with auto-merge on. |
| `sync-develop` | push to `main` | Merges `main` into `develop`. |
| `release` | push to `main` | Runs [release-please](https://github.com/googleapis/release-please): a release PR bumps the version and `CHANGELOG.md` from Conventional Commits; merging it tags and publishes the GitHub Release. |
| Dependabot | weekly | Updates GitHub Actions versions, with PRs against `develop`. |

A check that compiles a kernel is only evaluated in a PR and built on the push to `develop` or `main` after the merge.

## Checks

Every host gets its checks from the registry, so a new host reaches CI without touching YAML.

| Check | When | What it proves |
|---|---|---|
| `<host>-toplevel` | every host without an SD image | the system builds |
| `<host>-boots` | every host without an SD image | the VM reaches `multi-user.target` and every `home-manager-<user>` service |
| `<host>-sd-image` | Raspberry Pi hosts, in place of the two above | the SD card image builds |
| `<host>-disk` | hosts with `disk` | install on a blank disk, reboot, `/` wiped, btrbk snapshot taken; Secure Boot and TPM2 unlock on `x86_64-linux` hosts with `secure-boot` |
| `<host>-wipe-home` | hosts with `disk` but not `wipe-home` | the install test with `wipe-home` added |
| `<host>-nvidia` | `x86_64-linux` hosts without `nvidia` | the host with `nvidia` evaluates and its NVIDIA open kernel module builds |
| `<host>-installer` | hosts in `installer.testedHosts` | the installer ISO installs without questions and the installed disk boots |
| `profile-*`, `system-*`, `dev-*`, `home-*`, `desktop-*`, `app-*` | modules with a `check.nix` | the module's own VM test |
| `options-doc` | always | `docs/options.md` matches the registry options |

## Running the CI scripts locally

The matrix and scripts live in Nix (`modules/flake/ci/`), so they run the same on your machine.

| Command | What it does |
|---|---|
| `nix eval --json .#ci.matrix` | every check with its system and runner |
| `nix run .#ci-plan -- pull_request` | plan a run as CI does (`pull_request` or `push`), skipping what your store or substituters hold |
| `nix build -L .#checks.x86_64-linux.example-boots` | build one check |
| `nix run .#closure-diff -- <base rev or branch>` | package diff of the hosts in `nix eval --json .#ci.hosts` for the current platform |
| `nix run .#sync-develop` | merge `main` into `develop` and push, from a checkout of `develop` |
