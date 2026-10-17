# CI and repository settings

What the GitHub Actions workflows do, and what a fork sets so they run.

## Turn CI on in your fork

1. **Keep `develop`**: untick "Copy the main branch only" when forking, or create `develop` from `main`. Work lands on `develop`, then reaches `main` through a PR.
2. **Enable the workflows**: GitHub disables them on forks. Open the Actions tab and enable them.
3. **Bot GitHub App**: the release PR and the weekly lock and upstream PRs need an App token, since PRs opened with `GITHUB_TOKEN` trigger no workflow. [Create a GitHub App](https://github.com/settings/apps/new) with webhooks off and the repository permissions **Contents** and **Pull requests** set to read and write, generate a private key, install the App on your fork, then set the repository variable `BOT_CLIENT_ID` to its client ID and the secret `BOT_PRIVATE_KEY` to the key. One App serves all your repositories.
4. **Allow auto-merge and merge commits**: Settings, General, "Allow auto-merge" (the lock PR) and "Allow merge commits" (the upstream PRs).
5. **Ruleset on `main`**: require a pull request and the `ci-ok` status check. Without a required check, auto-merge merges at once.
6. **Ruleset on `develop`**: block deletions and force pushes, or "Automatically delete head branches" deletes `develop` when a `develop` into `main` PR merges.
7. **Binary cache (recommended)**: create a public [Cachix](https://www.cachix.org) cache, set the repository variable `CACHIX_CACHE` to its name and the secret `CACHIX_AUTH_TOKEN` to a read and write token. Every job pulls from it; only pushes to `main` and `develop` write what they built, except sources and images (SD card, ISO, disk, VM). A PR reads the cache but never gets the token, so code under review, such as an upstream sync PR, cannot poison it; the push after the merge builds and uploads what the PR built. Without it, every run builds every check and PRs never build kernels.

Scheduled workflows and Dependabot only run from the default branch, so they start once the workflows are on `main`.

## Workflows

| Workflow | Trigger | What it does |
|---|---|---|
| `check` | every PR, push to `main` and `develop` | Lints the flake, plans the build with `ci-plan` (checks already in the binary cache are skipped), builds each remaining check on a runner of its system (`ubuntu-24.04` with KVM, or `ubuntu-24.04-arm`), retrying a VM test once, computes the package diff ([nvd](https://git.sr.ht/~khumba/nvd)) per system on a PR, and writes one readable report ([below](#the-ci-report)). `ci-ok` sums up the result. |
| `update-flake-lock` | Monday 06:00 UTC, or by hand | Opens a PR updating `flake.lock` against `main`, with auto-merge on. |
| `sync-develop` | push to `main` | Merges `main` into `develop`. |
| `sync-upstream` | Monday 07:00 UTC, or by hand, forks only | Opens a PR merging the starter's latest release, for you to review and merge ([updates.md](updates.md)). |
| `release` | push to `main`, starter only | Runs [release-please](https://github.com/googleapis/release-please): a release PR bumps the version and `CHANGELOG.md` from Conventional Commits; merging it tags and publishes the GitHub Release. |
| Dependabot | weekly | Updates GitHub Actions versions, with PRs against `develop`. |

A check that compiles a kernel is only evaluated in a PR and built on the push to `develop` or `main` after the merge.

## The CI report

The `report` job runs at the end of every `check` run, even when a job failed or hit its time limit; a run cancelled, for instance by a newer push, writes no report. It writes its report to the run summary (the run's page in the Actions tab) and, on a PR from a branch of the same repository, to one PR comment that each new run rewrites. A PR from another fork gets the run summary only, since its token cannot comment.

The job runs `ci-report` from the base branch of the PR (the pushed commit on a push), never from the PR itself, and reads the machines and the check descriptions from the outputs of the `plan` job. A PR whose flake does not evaluate still gets its report: `The flake did not evaluate, so nothing was built or tested.`, with a pointer to the `plan` job log that names the file and line.

Line by line:

- **The verdict**, in bold: `Everything works: every machine builds, boots and passed its tests.`, or how many checks failed or did not finish and which machines would not get the change. A failure in a check that belongs to no machine still keeps CI red.
- `N tests passed only on their second try and are marked unstable.`: a VM test failed once, then passed on its retry. The run stays green, but a test that keeps showing up here is flaky.
- `The binary cache was unreachable: everything was built from scratch.`: Cachix did not answer, so the run was slower than usual.
- `Checks that compile a kernel run after the merge.`: a PR never compiles a kernel; the push to `develop` after the merge builds those checks.
- `N checks were skipped: they already passed with the same code.`: their result is already in the binary cache.
- **One table per machine**, with each check, what it proves and its result: `passed`, `failed`, `unstable`, `did not finish` (the job was cancelled or hit its time limit before its first attempt ended), or `skipped` with the reason. A retry cut by the time limit keeps the `failed` result and the excerpt of the first attempt. A check that runs on two systems gets one row per system, with the system after its name.
- **Module checks**, folded, with the same columns; the block opens by itself when one of them failed.
- **What went wrong**: for each failed check, the likely cause and what to do, read from `modules/flake/ci/causes.tsv` (the first row, top to bottom, whose pattern matches a line of the excerpt wins), the excerpt of the log (the last 20 distinct lines that name an error, each with the two lines before it), and a link to the full log.
- **Package changes per machine**, folded, on a PR: the nvd diff of each host against the base branch.

A comment over 64 KiB is cut and points to the run summary, which holds up to 1 MiB.

## Checks

Every host gets its checks from the registry, so a new host reaches CI without touching YAML.

| Check | When | What it proves |
|---|---|---|
| `<host>-toplevel` | every host without an SD image | the system builds |
| `<host>-boots` | every host without an SD image | the VM reaches `multi-user.target` and every `home-manager-<user>` service |
| `<host>-sd-image` | Raspberry Pi hosts, in place of the two above | the SD card image builds |
| `<host>-disk` | hosts with `disk` | install on a blank disk, reboot, `/` wiped, btrbk snapshot taken; Secure Boot and TPM2 unlock on `x86_64-linux` hosts with `secure-boot`; the swap file, with a 1G size, on hosts with `hibernate` |
| `<host>-wipe-home` | hosts with `disk` but not `wipe-home` | the install test with `wipe-home` added |
| `<host>-installer` | hosts in `installer.testedHosts` | the installer ISO installs without questions and the installed disk boots |
| `base-*`, `feature-*`, `profile-*`, `app-*` | modules with a `check.nix` | the module's own VM test |
| `wrong-namespace` | always, on `x86_64-linux` | a host naming in `features`, `profile` or `hardware` a name of another field fails to evaluate, with an error naming that field and listing only its own names; a name in `apps` that is neither an app nor a nixpkgs package lists the configured apps |
| `apps-once` | always, on `x86_64-linux` | apps reached twice (`shell` and `zsh`, `hyprland` and `lock`, on the host and the user) build the same system as each one once, with one hyprlock PAM service |
| `flake-modules` | always, on `x86_64-linux` | no name is published by two namespaces in the `flake.modules.nixos` output |
| `facter` | always, on `x86_64-linux` | a host with the registry field `facter` set to nixpkgs' sample report turns on nixpkgs' facter module |
| `options-doc` | always | `docs/options.md` matches the registry options |
| `init` | while the `example` host exists | `nix run .#init -- --yes` on a copy of the flake writes a host that parses, with the answers and features asked, removes the examples and refuses to overwrite a host |
| `example-hosts` | while an example host exists, on `x86_64-linux` | every example host has a line, enabled or commented out, for every registry field, profile, feature, hardware module and app of the starter |
| `ci-report` | always, on `x86_64-linux` | `ci-build` keeps a clean log excerpt and retries a VM test once; `ci-report` turns sample runs into the right verdict, a table per host that never mixes two hosts, the cause of a failure, and a comment cut to fit |

## Running the CI scripts locally

The matrix and scripts live in Nix (`modules/flake/ci/`), so they run the same on your machine.

| Command | What it does |
|---|---|
| `nix eval --json .#ci.matrix` | every check with its system, runner and `retry` (true for a NixOS VM test) |
| `nix run .#ci-plan -- pull_request` | plan a run as CI does (`pull_request` or `push`), skipping what your store or substituters hold |
| `nix build -L .#checks.x86_64-linux.example-boots` | build one check |
| `nix run .#ci-build -- x86_64-linux example-boots results --retry` | build one check as CI does: with `--retry`, a failure is rebuilt once and a pass on the second attempt is `unstable`; writes `results/x86_64-linux-example-boots.json` with the outcome, the attempts and an excerpt of the failing log, after each failed attempt and at the end |
| `nix eval --json .#ci.hostChecks` | the checks of each host, the exact list the report groups them by |
| `nix eval --json .#ci.descriptions` | what each check proves, as the report says it |
| `HOST_CHECKS=$(nix eval --json .#ci.hostChecks) DESCRIPTIONS=$(nix eval --json .#ci.descriptions) RESULTS_DIR=results nix run .#ci-report` | write the report of the results in `results/` to stdout; `PLANNED`, `SKIPPED`, `HOST_CHECKS` and `DESCRIPTIONS` take the `checks`, `skipped`, `hostChecks` and `descriptions` JSON of `ci-plan`, `CACHE_OUTCOME`, `LINT_RESULT` and `PLAN_RESULT` the outcome of the cache step, the lint job and the plan job, `RUN_URL` the link to the run; `--comment` cuts it to fit a PR comment |
| `nix run .#closure-diff -- <base rev or branch>` | package diff of the hosts in `nix eval --json .#ci.hosts` for the current platform |
| `nix run .#sync-develop` | merge `main` into `develop` and push, from a checkout of `develop` |
