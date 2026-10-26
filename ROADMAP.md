# Roadmap

Each lot builds and boots on its own before the next starts. Each one keeps a guardrail in CI: a change that touches a lot keeps its checks green. Everything is developed and tested in VMs.

## Shipped

One line per lot; [CHANGELOG.md](CHANGELOG.md) has the details of each release.

| Lot | Guardrail |
|---|---|
| **1. Repo tooling**: dendritic flake, nixfmt, statix, deadnix, CI with KVM, PR package diff, weekly lock update, release-please | `treefmt`, `ci-ok` required on `main` |
| **2. Host registry**: `configurations.nixos.<name>` generates the config, its checks and its CI entry | `<host>-toplevel`, `<host>-boots`, `wrong-namespace` |
| **3. Disk**: disko layout, LUKS2, btrfs subvolumes, `/` rolled back at every boot, state declared in `/persist` | `<host>-disk` |
| **4. Boot**: opt-in `secure-boot` with lanzaboote, TPM2 unlock enrolled on its own (PCR 7 and 15), passphrase kept | `<host>-disk` on a `secure-boot` host |
| **5. Hardware**: `firmware`, `audio`, `bluetooth`, GPU modules, nixos-hardware profiles, unfree packages allowed by name | `<host>-toplevel` of hosts importing them |
| **6. Performance profiles**: `desktop`, `laptop`, `server` | `profile-desktop`, `profile-laptop`, `profile-server` |
| **7. Services**: systemd-resolved, btrbk snapshots, opt-in `ssh` and `secrets` (sops-nix) | `base-resolved`, `feature-ssh`, `feature-secrets`, `<host>-disk` |
| **8. Dev base**: `nh clean`, nix-index with comma, opt-in rootless `docker`, `libvirt`, `nix-ld`, `direnv` | `base-nix`, `app-nix-index`, `feature-docker`, `feature-libvirt` |
| **9a. Shell**: zsh, starship, git, neovim and CLI tools through home-manager, XDG layout, opt-in `stow` | `app-zsh`, `app-stow` |
| **9b. Wipe `/home`**: opt-in `wipe-home`, previous home kept 7 days, declared paths restored from `/persist` | `example-disk` and `example-server-disk`, whose hosts list `wipe-home`; in a fork, `<host>-wipe-home` for a host with `disk` that does not |
| **10. Desktop**: `greetd`, `plymouth`, `hyprland` with waybar, fuzzel, kitty, mako, hyprlock and hypridle | `app-hyprland`, `feature-plymouth` |
| **11. Raspberry Pi**: `raspberry-pi-4` and `raspberry-pi-5` on nixos-raspberrypi, `pi` profile, SD image | `<host>-sd-image`, `profile-pi` |
| **12. One file per machine**: registry fields in plain values, `apps/` layer, generated `docs/reference/options.md` | `app-firefox`, `options-doc` |
| **13. Fast CI**: only checks missing from the binary cache are built, no kernel built in a PR | `ci-plan` |
| **14. Installer ISO**: `nix build .#installer-<host>` for every host with `disk`, `install-host` assistant on tty1 | `example-server-installer` |
| **15. Updates**: host `flake` field, `update` command, opt-in `auto-upgrade` | `base-update` |
| **16. Hardware report**: host `facter` field reading the installer's nixos-facter report | `facter` |
| **17. Printing**: CUPS with driverless network printers | `feature-printing` |
| **18. Init**: commented example hosts, `nix run .#init` writes a fork's first host and drops the examples | `init`, `example-hosts` |
| **19. Hibernate**: opt-in `hibernate`, swap file in LUKS, resume before the `/` rollback, suspend-then-hibernate | `feature-hibernate` |
| **20. Doctor**: `doctor` reports Secure Boot, TPM2, snapshots, failed units, updates and what the next boot erases | `base-doctor`, `<host>-disk` |
| **21. CI report**: one report per run, a table per machine, the likely cause of a failure, a flaky VM test retried once until lot 43 | `ci-report` |
| **22. Registry layout**: `features/`, `apps/`, `users/`, `hosts/`, `flake/`; people defined once, with an account per host | `wrong-namespace`, `apps-once`, `flake-modules`, `example-hosts`, `init` |
| **23. Wiped home by default**: `example` lists `wipe-home`, every home keeps `~/.ssh` and `~/.gnupg`, and the `gh` app keeps `~/.config/gh` | `example-disk`, `example-server-disk`, `profile-desktop`, `feature-printing` |
| **24. Validated upgrades**: CI moves `deploy/<host>` once its checks pass, `auto-upgrade` applies only that version, from the fork or a mirror, and a generation that does not start is rolled back | `ci-promote`, `feature-auto-upgrade`, `feature-auto-upgrade-sources`, `feature-rollback`, `feature-rollback-secure-boot`, `ci-mirror` |
| **25. Upstream**: `nix run .#upstream` shows a fork the breaking changes and changelog, then prepares the merge PR; the weekly `sync-upstream` runs only when the fork sets `SYNC_UPSTREAM` | `upstream` |
| **26. Base defaults**: journal capped at 500 MB, AppImages run with `nix-ld`, CPU microcode with `firmware` | `base-journald`, `feature-nix-ld`, `example-server-toplevel` (imports `firmware`), `example-hosts` |
| **27. Upgrade feedback**: `doctor` says what the last auto-upgrade did, `doctor --history`, one desktop notification per upgrade | `app-upgrade-notify`, `base-doctor` |
| **28. Try the install**: `nix run .#try-install -- <host>` runs the installer ISO in a local VM with Secure Boot setup mode and a TPM2 | `try-install` |
| **29. Update path proofs**: boot counting proven on the locked systemd and lanzaboote; an upgrade from the base revision keeps the machine's state | `feature-rollback`, `feature-rollback-secure-boot`, `feature-rollback-failed-unit`, `feature-migration`, `feature-migration-without-disk` |
| **30. CI evidence**: boot time beside the base's, VM test screenshots, a nightly `reproducibility` run never required by `ci-ok` | `ci-report`, `reproducibility`, `example-boots`, `example-server-boots` |
| **31. Signed deploy refs**: CI signs each promotion of `deploy/<host>`, a host with `signers` applies only a commit signed for it, `init` generates the key pair | `ci-promote`, `feature-auto-upgrade-signed`, `base-doctor`, `init` |
| **32. Live badges**: checks passing, VM tests, hosts built, boot time and reproducibility, published to an orphan `badges` branch | `ci-badges` |
| **33. Standalone apps**: no app brings another app or a feature; Hyprland binds only the apps and audio it has; hardware and printing leave the profiles | `hyprland-alone`, `feature-printing`, `profile-desktop`, `profile-laptop`, `apps-once`, `app-hyprland`, `app-zsh`, `app-starship`, `app-git`, `app-delta`, `app-neovim`, `app-fzf`, `app-eza`, `example-hosts`, `init` |
| **34. Static report text**: a fixed sentence under the report's verdict; release-please writes the release notes | `ci-report`, `upstream` |
| **35. Dual boot**: opt-in `dual-boot` starts Windows from the systemd-boot menu through the firmware, on its own disk or beside NixOS, Secure Boot and BitLocker intact | `feature-dual-boot`, `feature-dual-boot-same-disk`, `feature-dual-boot-systemd-boot`, `feature-dual-boot-hook-failure`, `example-hosts`, `init`, `example-disk` |
| **36. Variants**: host field `specialisations.<name>`, kept by `update` and `auto-upgrade`; `steam`, `gamemode` and `mangohud` apps | `specialisations`, `feature-auto-upgrade`, `wrong-namespace`, `example-hosts` |
| **37. Base is mechanism only**: `base` keeps what every fork needs; `nix-index` and `gh` become apps, zram moves to the profiles, policy values become `mkDefault` | `base-nix`, `base-resolved`, `app-nix-index`, `app-gh`, `profile-desktop`, `profile-laptop`, `profile-pi`, `example-hosts` |
| **38. Fork overrides**: a fork overrides every shipped policy value with a plain definition, without `mkForce` | `fork-overrides`, `example-hosts` |
| **39. FIDO2**: opt-in `fido2`: a security key logs in, runs `sudo` and, once enrolled, unlocks the disk | `feature-fido2` on an emulated CanoKey, `apps-once`, `example-hosts`, `init` |
| **40. Measured performance in CI**: on a PR, the base and the new generation boot in turn, and the report flags boot time, idle RAM, closure size or services past a threshold; never required by `ci-ok` | `perf-regression`, `ci-report` |
| **41. Docs a fork owns**: `docs/` follows Diataxis behind `docs/README.md`, names no example outside `docs/maintenance/`, every module has a section; the `docs` check resolves links offline and the `init` check runs it on its forks | `docs`, `init`, `options-doc` |
| **42. Benchmarks in the CI VM**: on a PR, each boot of the base and of the new generation also runs CPU (`sysbench cpu`), memory bandwidth (`sysbench memory`) and disk (`fio`, 4k random read) benchmarks with the same binaries on both sides, and the report flags a median that drops past its threshold; never required by `ci-ok` | `perf-regression`, `ci-report` |
| **43. Reliable CI**: every check passes or fails for a reason, never by chance: no VM falls back to network boot, tests wait for a unit, a log line or a file within their `globalTimeout`, a failed cache download is told apart from a failing build, the nightly `flaky` workflow reports the pass rate of every VM check (230 of 230 runs passed), and a check is built once, without the retry of lot 21 | `vm-launchers`, `ci-flaky`, `ci-report`, `ci-plan`, nightly `flaky` |
| **44. Rust workspace and `forge` CLI**: a Cargo workspace in `crates/`, built with crane and the nixpkgs toolchain; clippy pedantic, rustfmt, cargo-deny, cargo-audit and nextest as flake checks; one `forge` binary with subcommands, on every host. Forks add their own `forge-<name>` commands by listing packages in `forge.commands`, called with a versioned JSON contract, without a recompile. First port: `check-docs`, lychee replaced by an offline Rust link and anchor check | `rust-clippy`, `rust-fmt`, `rust-deny`, `rust-audit`, `rust-tests`, `docs`, `init`, `base-forge` |
| **45. `forge doctor`**: `doctor.sh` replaced by the Rust `forge doctor`, which reads the host's settings from a JSON file each system's `forge` points to; forks add checks with `forge` commands that declare `"doctor":true` and print JSON lines, run in their own process group and stopped after 30 s | `rust-tests`, `rust-clippy`, `base-doctor`, `base-forge`, the `<host>-disk` tests, `feature-rollback`, `feature-migration` |
| **46. `forge update` and `forge auto-upgrade`**: `update.sh` and `auto-upgrade.sh` replaced by Rust built-ins sharing doctor's command port; the service runs the system's wrapped `forge` with its settings in a JSON file, trusts a checkout through file descriptors opened without following links, bounds each source at 60 s, and decides from a pure function covered by Rust tests | `rust-tests`, `rust-clippy`, `base-update`, `base-doctor`, `base-forge`, `feature-auto-upgrade`, `feature-auto-upgrade-signed`, `feature-auto-upgrade-sources`, `feature-rollback`, `feature-rollback-secure-boot`, `feature-rollback-failed-unit`, `app-upgrade-notify`, `example-disk` |

## Planned

In order, each with its guardrail. Every lot starts with a state of the art review (current tools and practices, with sources) and ships to that standard, not the first thing that works. Each Rust lot keeps the NixOS tests unchanged as proof of the same behavior, replaces the tool's shell tests with Rust tests and deletes the `.sh` in the same lot; scripts under about 20 lines stay shell.

| Lot | Guardrail |
|---|---|
| **47. `forge install`** replaces `install-host`, `tpm2-enroll` and `fido2-enroll` | `<host>-installer`, `<host>-disk`, `feature-fido2` |
| **48. `forge dual-boot`** replaces `dual-boot.sh` and `beside-windows.sh` | `feature-dual-boot`, `feature-dual-boot-same-disk`, `feature-dual-boot-systemd-boot`, `feature-dual-boot-hook-failure` |
| **49. `forge init` and `forge upstream`** | `init`, `upstream` |
| **50. CI scripts and the report in Rust**: `ci-build`, `ci-plan`, `ci-promote`, `ci-badges`, `ci-report` with its jq, the mirror and reproducibility; snapshot tests replace the `*-test.sh` | `ci-report`, `ci-plan`, `ci-promote`, `ci-badges`, `ci-mirror`, `reproducibility` |
| **51. `stow-dotfiles` and `upgrade-notify`** in Rust | `app-stow`, `app-upgrade-notify` |

## Later

Kept for when a fork asks for them; none is planned yet.

| Idea | Guardrail |
|---|---|
| **Benchmarks on real hardware**: a self-hosted runner on a real machine benchmarks each night generation N against N-1 with the Phoronix Test Suite, games with a built-in benchmark (average FPS and 1 % low through MangoHud) and scheduler latency (`cyclictest`). `forge bench` runs the same suite on any fork's hardware, and under the `dual-boot` Windows of the same machine for a fair comparison. Results use a standard format that forks may publish, opt-in, to cover hardware upstream lacks. FPS and scheduler latency need real hardware: the VM has no GPU and the hypervisor schedules its vCPUs | nightly `bench` on the self-hosted runner, never required by `ci-ok` |
| **Variants as the machine's modes**: host field `defaultSpecialisation` makes a variant the default boot entry; the example hosts keep only what every mode shares (hardware, audio, browser) and show commented `dev` and `gaming` variants, each with its own desktop and terminal: `hyprland`, `kitty`, `direnv`, `docker` and `libvirt` for `dev`, a second desktop app such as `gnome` with `steam`, `gamemode` and `mangohud` for `gaming`; an app listed in one variant, home-manager part included, stays out of the others | `specialisations`, `specialisations-desktops`, `app-gnome`, `example-hosts`, `init` |
| **Hot switch between modes**: `forge update -s <variant>` applies a variant with another desktop without a reboot; it asks before closing an open graphical session, reads the `app-*` units uwsm started, stops the session, lets greetd log the user into the variant's desktop and relaunches the same `.desktop` entries there, so Firefox restores its tabs and Steam reconnects; window positions and what ran in a terminal are not kept; a variant whose kernel, kernel parameters, initrd or GPU driver differ from the booted system is refused with the reason and set for the next boot instead, as the example hosts' variant comment says | `specialisations-hot-switch`, from `hyprland` to the second desktop and back, the new compositor running with the apps relaunched |
| **Installer TUI**: a Rust TUI on `forge install` reads the nixos-facter report and proposes hardware, profile, `secure-boot`, TPM2, `fido2` and `dual-boot` from what it finds, with steps a fork extends | `<host>-installer` |
