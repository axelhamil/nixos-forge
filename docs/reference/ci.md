# CI reference

The workflows, every check and the CI scripts you can run locally. To turn CI on in your fork and read its report, see [how-to/ci.md](../how-to/ci.md).

The source of truth is the flake itself: `nix eval --json .#ci.descriptions` prints every check of your fork with the sentence the report shows for it, and `nix eval --json .#ci.matrix` lists the checks the `build` job runs, with their system and runner.

## Workflows

| Workflow | Trigger | What it does |
|---|---|---|
| `check` | every PR, push to `main` and `develop`; a newer push cancels the running check of its PR or of `develop`, never of `main` | Lints the flake, then plans the build with `ci-plan`: checks already in the binary cache are skipped. Builds each remaining check on a runner of its system (`ubuntu-24.04` with KVM, or `ubuntu-24.04-arm`) and retries a failed VM test once. On a PR, it also computes the package diff per system ([nvd](https://git.sr.ht/~khumba/nvd)) and measures the base revision against this one in the `perf` job. It ends with one report ([how-to/ci.md](../how-to/ci.md#the-ci-report)); `ci-ok` sums up the result, without waiting for `perf`. On a push to `main`, `promote` moves `deploy/<host>` for every machine whose checks all passed, and `badges` publishes the live badges. |
| `mirror` | push to `main` or `deploy/*`, when `MIRROR_URL` is set | Pushes `main` and every `deploy/<host>` to a second forge ([how-to/ci.md](../how-to/ci.md#mirror-to-a-second-forge)). |
| `reproducibility` | every day 03:00 UTC, or by hand | Rebuilds the system of every `x86_64-linux` machine and compares it ([below](#reproducibility)). On `main`, it publishes the `reproducibility` badge. Informational: `ci-ok` never waits for it. |
| `update-flake-lock` | Monday 06:00 UTC, or by hand | Opens a PR updating `flake.lock` against `main`, with auto-merge on. Its body lists the inputs that moved. |
| `sync-develop` | push to `main` | Merges `main` into `develop` with the App token, so `check` runs on the merged `develop`. |
| `sync-upstream` | Monday 07:00 UTC, or by hand; forks only, when the repository variable `SYNC_UPSTREAM` is `true` | Runs `nix run .#upstream -- --yes` on `main` and opens the PR that merges upstream's latest release ([updates.md](../explanation/updates.md#your-fork-follows-upstream)). The variable `UPSTREAM` (`owner/repo`) points it at another upstream repository. |
| `release` | push to `develop`, upstream only | Skipped in a fork. |
| Dependabot | weekly | Updates GitHub Actions versions, with PRs against `develop`. |

A check that compiles a kernel is only evaluated in a PR, and built on the push to `develop` or `main` after the merge. A newer push to `develop` cancels that build, and its own run starts it again, since nothing was cached. `main` never cancels, so a release always builds every kernel.

## Checks

Every host gets its checks from the registry, so a new host reaches CI without touching a workflow.

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
| `feature-dual-boot-same-disk` | always, on `x86_64-linux` | `install-beside-windows` refuses a 100M ESP, installs beside Windows on one disk, and the MSR, `C:` and recovery partition stay byte for byte the same |
| `feature-dual-boot-systemd-boot` | always, on `x86_64-linux` | `feature-dual-boot` without `secure-boot`, on plain systemd-boot |
| `feature-dual-boot-hook-failure` | always, on `x86_64-linux` | with lanzaboote, a boot loader install that fails makes the switch fail |
| `specialisations` | always, on `x86_64-linux` | a registry host switches to a variant, gets its features, apps, hardware and `module`, and switches back |

### Checks of the registry and the docs

| Check | When | What it proves |
|---|---|---|
| `treefmt` | every run, in the `lint` job | nixfmt, statix and deadnix find nothing |
| `wrong-namespace` | always, on `x86_64-linux` | a name given to the wrong field (`features`, `profile`, `hardware`, `apps`, `users`) fails to evaluate, with an error that names the field and lists its own names; in `apps`, a name that is neither an app nor a package of the host's pkgs, overlays included |
| `apps-once` | always, on `x86_64-linux` | an app named by both the host and the person is imported once, and an unfree package added by a host overlay and named in `apps` is installed and allowed |
| `fork-overrides` | always, on `x86_64-linux` | plain definitions in a host's `module` override the default policy values of the shipped modules without `mkForce`, `nvidia` with a nixos-hardware profile keeps that profile's NVIDIA choice, and a Raspberry Pi with `firmware` and `plymouth` evaluates |
| `hyprland-alone` | always, on `x86_64-linux` | `hyprland` alone evaluates and binds no key to an app or to audio the host lacks; with `kitty`, `fuzzel`, `lock` and the `audio` hardware module it binds them all |
| `flake-modules` | always, on `x86_64-linux` | no name is published by two namespaces |
| `facter` | always, on `x86_64-linux` | a host with `facter` set to a sample report turns on nixpkgs' facter module |
| `options-doc` | always | `docs/reference/options.md` matches the registry options |
| `docs` | always, on `x86_64-linux` | every relative link and `#anchor` of `README.md` and `docs/` resolves (lychee, offline); every page of `docs/` is in `docs/README.md`; no page outside `docs/maintenance/` names upstream's examples or repository, unless the line says `<!-- docs-check: allow <name> -->`; every app, feature, hardware module and profile has a heading or first table cell naming it. Violations listed in `modules/flake/docs/pending.txt` pass, and a listed one that no longer fails stops the check |

### Checks of the CI scripts

| Check | When | What it proves |
|---|---|---|
| `ci-plan` | always, on `x86_64-linux` | with stand-ins for `nix` and `nix-eval-jobs`, `ci-plan` builds every check the binary cache lacks, skips a check that compiles its system's host kernel in a pull request only, with a 240 minutes timeout on a push, never skips a check with a `baseInput` as cached, builds a system's closure only when no uncached check of it compiles its kernel, prints `checks=[]` when everything is cached, and stops naming the check that does not evaluate or is missing from the evaluation |
| `ci-promote` | always, on `x86_64-linux` | `deploy/<host>` moves only when every check of that host passed, never backwards, and with `DEPLOY_SIGNING_KEY` only to a promotion commit signed by that key; a tip the key did not sign is replaced with a warning |
| `ci-mirror` | always, on `x86_64-linux` | `main` and every `deploy/<host>` reach the mirror and nothing else does, a deploy branch moved back in the fork follows on the mirror, and the `MIRROR_TOKEN` header is sent |
| `ci-report` | always, on `x86_64-linux` | `ci-build` keeps a clean log excerpt and retries a VM test once; `ci-report` turns sample runs into the right verdict, tables, failure causes, boot times and performance sections, with the hosts left out of the performance comparison and their reason |
| `ci-badges` | always, on `x86_64-linux` | `ci-badges` turns sample runs into the right badges, and a publish keeps the badges it does not write |
| `reproducibility` | always, on `x86_64-linux` | with a stand-in for `nix`, `reproducibility` reports an identical rebuild, shows the diffoscope lines of a differing one without failing, and fails only when a system does not build |
| `try-install` | always, on `x86_64-linux` | `nix run .#try-install` refuses a device, an unknown host or a host without an installer before touching anything |
| `upstream` | always, on `x86_64-linux` | `nix run .#upstream` lists upstream's breaking changes and merges its release into a branch whose PR lists the upstream commits, or stops cleanly on a conflict |
| `perf-regression` | on a PR only, in the `perf` job, outside `ci-ok` and `ci.matrix`, while an `x86_64-linux` host boots in a VM here (forks included) | boots the base's and this version's VM of each host that also boots in a VM on the base, in turn, 5 times each, runs CPU, memory and disk benchmarks after each boot (`sysbench`, `fio`, from this version's `pkgs` on both sides), and writes `perf.json`, listing the other hosts as not measured (`absent` or `no-vm`); it fails only when a boot cannot be measured, never on a regression ([the report](../how-to/ci.md#performance-against-the-base)) |

The checks that only make sense upstream, on its example hosts and its upgrade path (`feature-migration`, `init`, `example-hosts`), are in [maintenance/ci.md](../maintenance/ci.md). <!-- docs-check: allow example -->

VM tests run on `x86_64-linux` only: they need KVM, and GitHub's arm64 Linux runners do not expose `/dev/kvm` ([actions/runner-images#14062](https://github.com/actions/runner-images/issues/14062)). Under emulation a boot test would not fit the 60 minute job limit. A fork with a self-hosted `aarch64-linux` runner exposing `/dev/kvm` can run them there.

## Reproducibility

`nix build --rebuild` builds the system derivation of a machine once more and compares the result with the first build, byte for byte. It rebuilds the last derivation only, the one that assembles the system, not every package under it. So it catches what that step adds (timestamps, ordering), not the reproducibility of nixpkgs. A difference now and then is expected: the job stays green, and its summary shows diffoscope's first 200 lines for each machine that differs. It fails only when a system does not build.

## Running the CI scripts locally

The matrix and scripts live in Nix (`modules/flake/ci/`), so they run the same on your machine.

| Command | What it does |
|---|---|
| `nix eval --json .#ci.matrix` | every check with its system, runner and `retry` (true for a NixOS VM test), except `treefmt` (the `lint` job) and `perf-regression` (listed by `nix eval --json .#ci.perfMatrix`) |
| `nix eval --json .#ci.descriptions` | what each check proves, as the report says it |
| `nix eval --json .#ci.hostChecks` | the checks of each host, the exact list the report groups them by |
| `nix run .#ci-plan -- pull_request` | plan a run as CI does (`pull_request` or `push`), skipping what your store or substituters hold |
| `nix build -L .#checks.x86_64-linux.<host>-boots` | build one check |
| `nix run .#ci-build -- x86_64-linux <host>-boots results --retry` | build one check as CI does and write `results/x86_64-linux-<host>-boots.json`: the outcome, the attempts, an excerpt of a failing log, the boot time and screenshots of a passing check. With `--retry`, a failure is rebuilt once and a pass on the second attempt is `unstable`. `--base-input <input> --base <flake-ref>` overrides that input of the check, as CI does for a check whose plan names a `baseInput`; a commit of all zeros keeps the locked input |
| `nix run .#ci-boot-time -- <flake ref> results/base` | read the boot time of every `<host>-boots` of `<flake ref>` that the binary cache or your store already holds, without building, into `results/base/boot-time.json`, which `ci-report` shows as the base |
| `nix run .#reproducibility [-- <host>...]` | rebuild each host's system (every host of your platform by default) and print a Markdown summary; exits 1 only when a system does not build |
| `HOST_CHECKS=$(nix eval --json .#ci.hostChecks) DESCRIPTIONS=$(nix eval --json .#ci.descriptions) RESULTS_DIR=results nix run .#ci-report` | write the report of the results in `results/` to stdout; `--comment` cuts it to fit a PR comment. `PLANNED` and `SKIPPED` take the `checks` and `skipped` JSON of `ci-plan`; `CACHE_OUTCOME`, `LINT_RESULT` and `PLAN_RESULT` the outcome of the cache step, the lint job and the plan job; `RUN_URL` the link to the run |
| `RESULTS_DIR=results HOST_CHECKS=... PLANNED=... SKIPPED=... LINT_RESULT=success nix run .#ci-badges -- checks badges` | write the shields endpoint files of a run to `badges/`; `reproducibility <summary.md> badges` writes `reproducibility.json`; `publish badges` commits `badges/` to the `badges` branch of `origin` |
| `HOST_CHECKS=$(nix eval --json .#ci.hostChecks) PLANNED='[...]' SKIPPED='[...]' nix run .#ci-promote -- results <sha>` | move each `deploy/<host>` of `origin` to `<sha>` when every check of that host in `results/` passed; `DEPLOY_SIGNING_KEY` takes the private key that signs the promotion commits |
| `MIRROR_TOKEN=<token> nix run .#ci-mirror -- <mirror url>` | push `main` and every `deploy/<host>` as `origin` held them at the last fetch, pruning the deploy branches `origin` no longer has; `MIRROR_TOKEN` is optional |
| `nix run .#closure-diff -- <base rev or branch>` | package diff of the hosts in `nix eval --json .#ci.hosts` for the current platform |
| `nix run .#sync-develop` | merge `main` into `develop` and push, from a checkout of `develop` |
