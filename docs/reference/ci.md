# CI reference

The workflows, every check and the CI scripts you can run locally. To turn CI on in your fork and read its report, see [how-to/ci.md](../how-to/ci.md).

The source of truth is the flake itself: `nix eval --json .#ci.descriptions` prints every check of your fork with the sentence the report shows for it, and `nix eval --json .#ci.matrix` lists the checks the `build` job runs, with their system and runner.

## Workflows

| Workflow | Trigger | What it does |
|---|---|---|
| `check` | every PR, push to `main` and `develop`; a newer push cancels the running check of its PR or of `develop`, never of `main` | Lints the flake, then plans the build with `ci-plan`: checks already in the binary cache are skipped. Builds each remaining check on a runner of its system (`ubuntu-24.04` with KVM, or `ubuntu-24.04-arm`), once each. On a PR, it also computes the package diff per system ([nvd](https://git.sr.ht/~khumba/nvd)) and measures the base revision against this one in the `perf` job. It ends with one report ([how-to/ci.md](../how-to/ci.md#the-ci-report)); `ci-ok` sums up the result, without waiting for `perf`. On a push to `main`, `promote` moves `deploy/<host>` for every machine whose checks all passed, and `badges` publishes the live badges. On every push, once the builds ended, `pin` pins in Cachix every host kernel it holds with `ci-pin-kernels`, so the cache never evicts what takes an hour to compile. |
| `mirror` | push to `main` or `deploy/*`, when `MIRROR_URL` is set | Pushes `main` and every `deploy/<host>` to a second forge ([how-to/ci.md](../how-to/ci.md#mirror-to-a-second-forge)). |
| `flaky` | every day 01:00 UTC, or by hand; upstream, or a fork whose repository variable `FLAKY` is `true` | Runs each `x86_64-linux` VM test whose derivation changed 5 times on the same commit and reports each one that did not pass all of its runs ([below](#flaky-vm-checks)). On `main`, it records the results in `flaky.json` on the `badges` branch. Informational: `ci-ok` never waits for it. |
| `reproducibility` | every day 03:00 UTC, or by hand; upstream, or a fork whose repository variable `REPRODUCIBILITY` is `true` | Rebuilds the system of every `x86_64-linux` machine and compares it ([below](#reproducibility)). On `main`, it publishes the `reproducibility` badge. Informational: `ci-ok` never waits for it. |
| `update-flake-lock` | Monday 06:00 UTC, or by hand, when `BOT_CLIENT_ID` is set | Opens a PR updating `flake.lock` against `main`, with auto-merge on. Its body lists the inputs that moved. |
| `sync-develop` | push to `main`, when `BOT_CLIENT_ID` is set | Merges `main` into `develop` with the App token, so `check` runs on the merged `develop`. |
| `sync-upstream` | Monday 07:00 UTC, or by hand; forks only, when the repository variable `SYNC_UPSTREAM` is `true` | Runs `nix run .#upstream -- --yes` on `main` and opens the PR that merges upstream's latest release ([updates.md](../explanation/updates.md#your-fork-follows-upstream)). The variable `UPSTREAM` (`owner/repo`) points it at another upstream repository. |
| `release` | push to `develop`, upstream only | Skipped in a fork. |
| Dependabot | weekly | Updates the pinned GitHub Actions in one grouped PR against `develop`, only to versions released at least 7 days ago. |

### Workflow security

The workflows follow GitHub's [security hardening guide](https://docs.github.com/en/actions/reference/security/secure-use), the [OpenSSF Scorecard](https://github.com/ossf/scorecard/blob/main/docs/checks.md) checks and the [zizmor](https://docs.zizmor.sh/audits/) audits:

- **Least privilege.** Each workflow grants `GITHUB_TOKEN` `contents: read` at most, or nothing where it uses the App token alone (`release`, `sync-develop`, `sync-upstream`); a write scope is set on the one job that needs it (`report`: `pull-requests: write`; `badges`, `reproducibility` and the `record` job of `flaky`: `contents: write`, to push the `badges` branch). App tokens ask for the permissions their job uses: `report` **Pull requests**, `promote` **Contents** and `update-flake-lock` **Contents** and **Pull requests**. `release`, `sync-develop` and `sync-upstream` take what the installation grants: asking for a permission the installation lacks fails the token request, and what they push to `main` or `develop` may change `.github/workflows/`, which needs **Workflows**; `.github/zizmor.yml` records these three exceptions.
- **Pinned actions.** Every action is pinned to a full commit SHA with its tag in a trailing comment, and Dependabot keeps both current.
- **Credentials.** `actions/checkout` keeps its token in the git config only in jobs that push (`promote`, `badges`, `reproducibility`, `release`, `sync-develop`, `sync-upstream`, the `record` job of `flaky`) or fetch the `badges` branch of a repository that may be private (the `plan` and `report` jobs of `flaky`, with a `contents: read` token), and says so with `persist-credentials: true`; every other job sets `false`.
- **No expression in a script.** `run:` lines read event data, matrix values and step outputs from `env:`, never from `${{ }}`.
- **Bounded runs.** Every job has a `timeout-minutes`; a newer run cancels an older `check` (except on `main`) or `reproducibility`, while the workflows that push serialize in their own `concurrency` group. Jobs whose variable is unset (`BOT_CLIENT_ID`, `CACHIX_CACHE` for `pin`, `MIRROR_URL`, `SYNC_UPSTREAM`, `FLAKY` and `REPRODUCIBILITY` in a fork) are skipped, never started.

`GH_TOKEN=$(gh auth token) nix run nixpkgs#zizmor -- .github/workflows .github/dependabot.yml` audits them, and reports nothing.

A check that compiles a kernel is only evaluated in a PR, and built on the push to `develop` or `main` after the merge. A newer push to `develop` cancels that build, and its own run starts it again, since nothing was cached. `main` never cancels, so a release always builds every kernel.

## Checks

Every host gets its checks from the registry, so a new host reaches CI without touching a workflow.

Every VM test has a time budget, its `globalTimeout`, about 3 times its longest green run in CI: 400 seconds for a module test, 800 for `<host>-boots`, 1400 for `<host>-installer`, 1500 for `<host>-disk` and `<host>-wipe-home`, 2000 per measured host for `perf-regression`, whose derivation timeout adds 10 minutes and whose `perf` job, from the `timeout` of `ci.perfMatrix`, 60. A test past its budget fails with `timeout reached; test terminating`, before the derivation and job limits, and the report says which wait never ended. A wait that can hang on something outside the test script (console text, a password prompt, the firmware, a long `nix copy` or install) has its own timeout below the budget, and no test waits for a fixed delay.

### Checks of each host

| Check | When | What it proves |
|---|---|---|
| `<host>-toplevel` | every host | the system builds, with its real revision; its closure is what `auto-upgrade` substitutes |
| `<host>-boots` | every host without an SD image | the VM reaches `multi-user.target` and every `home-manager-<user>` service; it records `systemd-analyze`'s total and userspace time in `boot-time.json`, and never fails because a boot is slow |
| `<host>-sd-image` | Raspberry Pi hosts, in place of `<host>-boots` | the SD card image builds |
| `<host>-disk` | hosts with `disk` | install on a blank disk, reboot, `/` wiped, a btrbk snapshot taken; with `secure-boot` on `x86_64-linux`, Secure Boot and the TPM2 unlock; with `hibernate`, a 1G swap file |
| `<host>-wipe-home` | hosts with `disk` but not `wipe-home` | the same install with `wipe-home` added: `/home` starts empty at every boot, the previous one is kept read-only in `/.snapshots`, declared paths come back |
| `<host>-installer` | hosts in `installer.testedHosts` only | the installer ISO installs the host without questions and the installed disk boots |

### Checks of the modules

| Check | When | What it proves |
|---|---|---|
| `base-*`, `feature-*`, `profile-*`, `app-*` | modules with a `check.nix`, on `x86_64-linux` | the module's own VM test: for instance `feature-hibernate` hibernates and resumes a machine, `feature-fido2` runs `sudo` and unlocks the disk with an emulated security key |
| `feature-rollback`, `feature-rollback-secure-boot` | always, on `x86_64-linux` | a new generation whose health check fails is never blessed: each of its 3 boots spends a try, then the machine boots the generation before, which keeps no counter. It runs on the locked systemd and lanzaboote, so a regression in boot counting fails it |
| `feature-rollback-failed-unit` | always, on `x86_64-linux` | the same with a service that fails after `multi-user.target`, which `auto-upgrade`'s `systemd-boot-check-no-failures` catches |
| `feature-auto-upgrade-signed` | always, on `x86_64-linux` | a machine with `signers` applies only a commit signed for it, takes it from a mirror when the first source serves an unsigned one, and refuses an older signed promotion replayed |
| `feature-auto-upgrade-sources` | always, on `x86_64-linux` | a host whose `mirrors` holds an `http://`, a `git://` or a relative URL fails to evaluate, naming each one; an `ssh://` or `user@host:path` one evaluates with a warning naming the root SSH key and known host it needs; `https://`, `file://` and absolute paths pass silently |
| `feature-dual-boot` | always, on `x86_64-linux` | under OVMF with Secure Boot and a TPM, a fake Windows on its own disk (the signed UEFI shell as `bootmgfw.efi`) starts from the menu through `BootNext`, with or without BitLocker, and its firmware entry is recreated when deleted |
| `feature-dual-boot-same-disk` | always, on `x86_64-linux` | `forge dual-boot install-beside-windows` refuses a 100M ESP, installs beside Windows on one disk, and the MSR, `C:` and recovery partition stay byte for byte the same |
| `feature-dual-boot-systemd-boot` | always, on `x86_64-linux` | `feature-dual-boot` without `secure-boot`, on plain systemd-boot |
| `feature-dual-boot-hook-failure` | always, on `x86_64-linux` | with lanzaboote, a boot loader install that fails makes the switch fail |
| `specialisations` | always, on `x86_64-linux` | a registry host switches to a variant, gets its features, apps, hardware and `module`, and switches back |

### Checks of the registry and the docs

| Check | When | What it proves |
|---|---|---|
| `treefmt` | every run, in the `lint` job | nixfmt, statix, deadnix and rustfmt find nothing |
| `wrong-namespace` | always, on `x86_64-linux` | a name given to the wrong field (`features`, `profile`, `hardware`, `apps`, `users`) fails to evaluate, with an error that names the field and lists its own names; in `apps`, a name that is neither an app nor a package of the host's pkgs, overlays included |
| `apps-once` | always, on `x86_64-linux` | an app named by both the host and the person is imported once, and an unfree package added by a host overlay and named in `apps` is installed and allowed |
| `fork-overrides` | always, on `x86_64-linux` | plain definitions in a host's `module` override the default policy values of the shipped modules without `mkForce`, `nvidia` with a nixos-hardware profile keeps that profile's NVIDIA choice, and a Raspberry Pi with `firmware` and `plymouth` evaluates |
| `hyprland-alone` | always, on `x86_64-linux` | `hyprland` alone evaluates and binds no key to an app or to audio the host lacks; with `kitty`, `fuzzel`, `lock` and the `audio` hardware module it binds them all |
| `flake-modules` | always, on `x86_64-linux` | no name is published by two namespaces |
| `vm-launchers` | on `x86_64-linux` | every test script under `modules/` that starts its own QEMU machine goes through `machine_command` of `helpers.testMachineScript`, which turns off QEMU's default network card so the firmware never boots over PXE: a call of a `create_*machine` launcher not defined under `modules/` (disko's `create_test_machine` included) must pass `start_command=machine_command(...)`; the check names each file and line that does not, and proves it on fixtures |
| `facter` | always, on `x86_64-linux` | a host with `facter` set to a sample report turns on nixpkgs' facter module |
| `options-doc` | always | `docs/reference/options.md` matches the registry options |
| `docs` | always, on `x86_64-linux` | every relative link and `#anchor` of `README.md` and `docs/` resolves (`forge check-docs`, offline); every page of `docs/` is in `docs/README.md`; no page outside `docs/maintenance/` names upstream's examples or repository, unless the line says `<!-- docs-check: allow <name> -->`; every app, feature, hardware module and profile has a heading or first table cell naming it. Violations listed in `modules/flake/docs/pending.txt` pass, and a listed one that no longer fails stops the check |

### Checks of the Rust code

The Cargo workspace `crates/` is built with [crane](https://github.com/ipetkov/crane) and the Rust toolchain of the locked nixpkgs; its dependencies are built once and shared by the package and these checks.

| Check | When | What it proves |
|---|---|---|
| `rust-clippy` | always, on `x86_64-linux` | `cargo clippy --all-targets` finds nothing, with `clippy::pedantic` on and every warning an error |
| `rust-fmt` | always, on `x86_64-linux` | `cargo fmt --check` changes nothing |
| `rust-deny` | always, on `x86_64-linux` | `cargo deny` (bans, licenses, sources, offline) accepts every dependency: a license of `crates/deny.toml`, crates.io only |
| `rust-audit` | always, on `x86_64-linux` | `cargo audit` finds no RustSec advisory against `Cargo.lock`, with the advisory database of the `advisory-db` input, which the weekly lock PR refreshes |
| `rust-tests` | always, on `x86_64-linux` | every Rust test passes under nextest, the insta snapshots included, never written in CI; among them `forge upstream` lists upstream's breaking changes and merges its release into a branch whose PR lists the upstream commits, or stops cleanly on a conflict, over real git repositories |

### Checks of the CI scripts

| Check | When | What it proves |
|---|---|---|
| `ci-pin-kernels` | always, on `x86_64-linux` | with stand-ins for `nix` and `cachix`, `ci-pin-kernels` pins each kernel output the cache holds under its name with `--keep-revisions 3`, reports and skips one the cache lacks, still tries every pin after one fails and then fails, and stops before any call without `CACHIX_AUTH_TOKEN` |
| `ci-mirror` | always, on `x86_64-linux` | `main` and every `deploy/<host>` reach the mirror and nothing else does, a deploy branch moved back in the fork follows on the mirror, and the `MIRROR_TOKEN` header is sent |
| `try-install` | always, on `x86_64-linux` | `nix run .#try-install` refuses a device, an unknown host or a host without an installer before touching anything |
| `perf-regression` | on a PR only, in the `perf` job, outside `ci-ok` and `ci.matrix`, while an `x86_64-linux` host boots in a VM here (forks included) | boots the base's and this version's VM of each host that also boots in a VM on the base, in turn, 5 times each, runs CPU, memory and disk benchmarks after each boot (`sysbench`, `fio`, from this version's `pkgs` on both sides), and writes `perf.json`, listing the other hosts as not measured (`absent` or `no-vm`); it fails only when a boot cannot be measured, never on a regression ([the report](../how-to/ci.md#performance-against-the-base)) |

### What `rust-tests` proves about the CI scripts

The CI scripts above are the `forge-ci` binary (`crates/forge-ci`), each wrapped by its `nix run .#<script>` package. Its Rust tests, run by `rust-tests` with stand-ins for `nix`, `nix-eval-jobs` and `diffoscope` and real git repositories, prove:

| Script | What it proves |
|---|---|
| `ci-plan` | with stand-ins for `nix` and `nix-eval-jobs`, `ci-plan` builds every check the binary cache lacks, skips a check that compiles its system's host kernel in a pull request only, with a 240 minutes timeout on a push, never skips a check with a `baseInput` as cached, builds a system's closure only when no uncached check of it compiles its kernel, prints `checks=[]` when everything is cached, and stops naming the check that does not evaluate or is missing from the evaluation |
| `ci-promote` | `deploy/<host>` moves only when every check of that host passed, never backwards, and with `DEPLOY_SIGNING_KEY` only to a promotion commit signed by that key; a tip the key did not sign is replaced with a warning |
| `ci-report` | `ci-build` builds a check once and keeps a clean log excerpt of a failure; `ci-report` turns sample runs into the right verdict, tables, failure causes, boot times and performance sections, with the hosts left out of the performance comparison and their reason |
| `ci-badges` | `ci-badges` turns sample runs into the right badges, a publish keeps the badges it does not write, a read prints a published file, and both stop when the `badges` branch cannot be fetched |
| `ci-flaky` | with stand-ins for `nix` and a local `origin`, `ci-flaky` records every run of a check without stopping at a failure, counts a rebuild whose output differs as a pass, never counts the first build and calls `nix build --rebuild` only once the check is realised; `ci-flaky-plan` plans nothing and without evaluating the flake on a tree already measured green, skips a derivation recorded with 5 passing runs, measures a failing or missing one again, measures everything when `flaky.json` is missing or malformed, and stops, saying so, when the `badges` branch cannot be fetched; `ci-flaky-report` lists every check, the skipped ones as not rerun, fails on a check under 100 % or without result, and records the tree only when every check passed |
| `reproducibility` | with a stand-in for `nix`, `reproducibility` reports an identical rebuild, shows the diffoscope lines of a differing one without failing, and fails only when a system does not build |

The checks that only make sense upstream, on its example hosts and its upgrade path (`feature-migration`, `init`, `example-hosts`), are in [maintenance/ci.md](../maintenance/ci.md). <!-- docs-check: allow example -->

VM tests run on `x86_64-linux` only: they need KVM, and GitHub's arm64 Linux runners do not expose `/dev/kvm` ([actions/runner-images#14062](https://github.com/actions/runner-images/issues/14062)). Under emulation a boot test would not fit the 60 minute job limit. A fork with a self-hosted `aarch64-linux` runner exposing `/dev/kvm` can run them there.

## Reproducibility

`nix build --rebuild` builds the system derivation of a machine once more and compares the result with the first build, byte for byte. It rebuilds the last derivation only, the one that assembles the system, not every package under it. So it catches what that step adds (timestamps, ordering), not the reproducibility of nixpkgs.

A difference now and then is expected. The job stays green, and its summary shows diffoscope's first 200 lines for each machine that differs. It fails only when a system does not build.

## Flaky VM checks

The nightly `flaky` workflow measures how often each VM test passes on code that does not change. `nix eval --json .#ci.flakyMatrix` lists the checks it may run: every NixOS VM test (the checks whose `vm` is true in `ci.matrix`, all on `x86_64-linux` today), each with its system and its runner. It leaves out each check's `drvPath`, which would make one evaluation hold every VM test at once: `plan` reads them from `nix-eval-jobs`, as `ci-plan` does, and adds them to the matrix it prints.

It runs upstream, and in a fork only when the repository variable `FLAKY` is `true`; otherwise the `if:` of the `plan` job, its only gate, skips every job. It measures only what changed, from the record `flaky.json` on the `badges` branch: the tree of the last commit measured with every check green, and the runs of each derivation.

- When the checked-out tree is the one recorded, `plan` stops before evaluating anything and the other jobs are skipped.
- Otherwise, a check whose `drvPath` is recorded with 5 passing runs is not rerun. A derivation that failed, or wrote no result, is measured again every night until it passes.
- A missing or malformed `flaky.json` is ignored: every check is measured. A `badges` branch that cannot be fetched stops `plan` and `report` with that reason, since measuring every check again would cost a night after a lock update.

Each check to measure gets its own job, with up to 300 minutes. The job realises the check once, from the binary cache or by building it, and that first build is not counted. It then runs `nix build --rebuild` 5 times, so the test itself runs again each time on the same inputs, and never stops at a failure. A rebuild whose output differs from the first one (a screenshot, a boot time) still passed. When the check could not be realised, each run builds it plainly until one passes.

The `report` job writes a table to the run summary: every check, its pass rate and its median time, marked `(not rerun, unchanged)` when it comes from the record. Below it, one sentence names each check under 100 % or without result, or says `Every VM check passed all of its runs.` The job fails only in the first case. On `main`, the `record` job, the only one allowed to write, publishes the new `flaky.json` to the `badges` branch; it runs only when `report` uploaded one, so a report that could not read the record, or stopped before writing it, never replaces it.

A night with no change costs one plan job of about 2 minutes. A night after a lock update changes every derivation and runs all 46 VM tests of upstream, about 1,000 runner minutes: about 180 minutes per round of every test, 5 rounds, plus each job's setup; the longest job, `feature-rollback-secure-boot`, takes about 2 and a half hours.

## Running the CI scripts locally

The matrix lives in Nix (`modules/flake/ci/`) and the scripts in the `forge-ci` binary (`crates/forge-ci`), wrapped by the packages of `modules/flake/ci/`, so they run the same on your machine.

| Command | What it does |
|---|---|
| `nix eval --json .#ci.matrix` | every check with its system, runner and `vm` (true for a NixOS VM test), except `treefmt` (the `lint` job) and `perf-regression` (listed by `nix eval --json .#ci.perfMatrix`, with its `baseInput` and its job `timeout` in minutes) |
| `nix eval --json .#ci.descriptions` | what each check proves, as the report says it |
| `nix eval --json .#ci.hostChecks` | the checks of each host, the exact list the report groups them by |
| `nix eval --json .#ci.kernelPins` | every output of every host kernel, each with its pin name `kernel-<host>-<output>` and its store path, evaluated without building |
| `nix run .#ci-plan -- pull_request` | plan a run as CI does (`pull_request` or `push`), skipping what your store or substituters hold |
| `nix build -L .#checks.x86_64-linux.<host>-boots` | build one check |
| `nix run .#ci-build -- x86_64-linux <host>-boots results` | build one check once, as CI does, and write `results/x86_64-linux-<host>-boots.json`: the outcome, an excerpt of a failing log, the boot time and screenshots of a passing check. `--base-input <input> --base <flake-ref>` overrides that input of the check, as CI does for a check whose plan names a `baseInput`; a commit of all zeros keeps the locked input. CI passes them as environment instead: `BASE_INPUT` with `BASE_SHA` for `--base-input $BASE_INPUT --base github:$GITHUB_REPOSITORY/$BASE_SHA`, an empty `BASE_INPUT` overriding nothing |
| `nix run .#ci-boot-time -- <flake ref> results/base` | read the boot time of every `<host>-boots` of `<flake ref>` that the binary cache or your store already holds, without building, into `results/base/boot-time.json`, which `ci-report` shows as the base |
| `nix eval --json .#ci.flakyMatrix` | the checks the `flaky` workflow may run, with their system and runner; `ci-flaky-plan` adds each `drvPath` from `nix-eval-jobs` |
| `nix run .#ci-flaky-plan` | print the `checks` (to run, with `runs`) and `all` outputs of the `flaky` plan, from the record on the `badges` branch of `origin` |
| `nix run .#ci-flaky -- x86_64-linux <check> 5 results` | realise one check, then rebuild it 5 times and write `results/x86_64-linux-<check>.json`: `{ check, system, runs: [{ passed, seconds }] }`; it always exits 0 once the runs are done |
| `MATRIX=<all> PLANNED=<checks> nix run .#ci-flaky-report -- results history` | print the Markdown table and sentences of the `flaky` workflow from the results in `results/` and the record of the `badges` branch, and write the new record to `history/flaky.json`; exits 1 when a check did not pass all of its runs or wrote no result |
| `nix run .#reproducibility [-- <host>...]` | rebuild each host's system (every host of your platform by default) and print a Markdown summary; exits 1 only when a system does not build |
| `HOST_CHECKS=$(nix eval --json .#ci.hostChecks) DESCRIPTIONS=$(nix eval --json .#ci.descriptions) RESULTS_DIR=results nix run .#ci-report` | write the report of the results in `results/` to stdout; `--comment` cuts it to fit a PR comment. `PLANNED` and `SKIPPED` take the `checks` and `skipped` JSON of `ci-plan`; `CACHE_OUTCOME`, `LINT_RESULT` and `PLAN_RESULT` the outcome of the cache step, the lint job and the plan job; `RUN_URL` the link to the run |
| `RESULTS_DIR=results HOST_CHECKS=... PLANNED=... SKIPPED=... LINT_RESULT=success nix run .#ci-badges -- checks badges` | write the shields endpoint files of a run to `badges/`; `reproducibility <summary.md> badges` writes `reproducibility.json`; `publish badges` commits `badges/` to the `badges` branch of `origin`; `read flaky.json` prints that file of the branch, and exits 3 when the branch or the file is missing, 1 when the branch cannot be fetched |
| `HOST_CHECKS=$(nix eval --json .#ci.hostChecks) PLANNED='[...]' SKIPPED='[...]' nix run .#ci-promote -- results <sha>` | move each `deploy/<host>` of `origin` to `<sha>` when every check of that host in `results/` passed; `DEPLOY_SIGNING_KEY` takes the private key that signs the promotion commits |
| `CACHIX_AUTH_TOKEN=<token> nix run .#ci-pin-kernels -- <cache>` | fetch with `nix build` every path of `ci.kernelPins` the Cachix cache `<cache>` holds, with its references from any substituter, since `cachix pin` needs the path valid in the local store, then pin it there, keeping 3 revisions of each pin, and print `pinned` or `not in <cache>, not pinned` for each (a nixpkgs kernel comes from cache.nixos.org and is never pushed there); exits 1 when a pin fails, after trying the others |
| `MIRROR_TOKEN=<token> nix run .#ci-mirror -- <mirror url>` | push `main` and every `deploy/<host>` as `origin` held them at the last fetch, pruning the deploy branches `origin` no longer has; `MIRROR_TOKEN` is optional |
| `nix run .#closure-diff -- <base rev or branch>` | package diff of the hosts in `nix eval --json .#ci.hosts` for the current platform; with `BUILD=false` (or `--no-build`), as the `closure` job sets it when a kernel is not cached, it only says why nothing was built |
| `nix run .#sync-develop` | merge `main` into `develop` and push, from a checkout of `develop` |
