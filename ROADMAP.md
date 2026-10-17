# Roadmap

Each lot builds and boots on its own before the next starts, and keeps a guardrail in CI: a change that touches a lot keeps its checks green. Anything beyond the shared base ships as an opt-in module with neutral defaults; personal choices belong in forks. Everything is developed and tested in VMs.

## Shipped

| Lot | Guardrail |
|---|---|
| **1. Repo tooling**: dendritic flake, nixfmt, statix, deadnix, CI matrix with KVM, PR package diff, weekly lock update, release-please | `treefmt`, `ci-ok` required on `main` |
| **2. Host registry**: `configurations.nixos.<name>` generates the config, its checks and its CI entry; each field resolves in the folder of its name | `<host>-toplevel`, `<host>-boots`, `wrong-namespace` |
| **3. Disk**: disko layout, LUKS2, btrfs subvolumes, `/` rolled back at every boot, state declared in `/persist` | `<host>-disk` |
| **4. Boot**: opt-in `secure-boot` with lanzaboote, TPM2 unlock enrolled on its own (PCR 7 and 15), passphrase kept | `<host>-disk` on a `secure-boot` host |
| **5. Hardware**: `firmware`, `audio`, `bluetooth`, `nvidia`, `amd-gpu`, `intel-gpu`, nixos-hardware profiles, unfree packages allowed by name | `<host>-toplevel` of hosts importing them |
| **6. Performance profiles**: `desktop`, `laptop`, `server` (kernel, sched-ext, power, zram, oomd) | `profile-desktop`, `profile-laptop`, `profile-server` |
| **7. Services**: systemd-resolved, btrbk snapshots, opt-in `ssh` and `secrets` (sops-nix) | `base-resolved`, `feature-ssh`, `feature-secrets`, `<host>-disk` |
| **8. Dev base**: `nh clean`, nix-index with comma, opt-in rootless `docker`, `libvirt`, `nix-ld`, `direnv` | `base-nix`, `base-nix-index`, `feature-docker`, `feature-libvirt` |
| **9a. Shell**: the `shell` app (zsh, starship, git, neovim, CLI tools) through home-manager, XDG layout, opt-in `stow` | `app-shell`, `app-stow` |
| **9b. Wipe `/home`**: opt-in `wipe-home`, previous home kept 7 days, declared paths restored from `/persist` | `<host>-wipe-home` |
| **10. Desktop**: `greetd`, `plymouth`, `hyprland` with waybar, fuzzel, kitty, mako, hyprlock and hypridle | `app-hyprland`, `feature-plymouth` |
| **11. Raspberry Pi**: `raspberry-pi-4` and `raspberry-pi-5` on nixos-raspberrypi, `pi` profile, SD image | `<host>-sd-image`, `profile-pi` |
| **12. One file per machine**: registry fields in plain values, users generated from the host, `apps/` layer with `firefox`, generated `docs/options.md` | `app-firefox`, `options-doc` |
| **13. Fast CI**: only checks missing from the binary cache are built, no kernel built in a PR | `ci-plan` |
| **14. Installer ISO**: `nix build .#installer-<host>` for every host with `disk`, `install-host` assistant on tty1 | `example-server-installer` |
| **15. Updates**: host `flake` field, `update` command, opt-in weekly `auto-upgrade`, weekly `sync-upstream` PR in forks, merged by hand | `base-update` |
| **16. Hardware report**: host `facter` field reading the installer's nixos-facter report | `facter` |
| **17. Printing**: CUPS with driverless network printers in the `desktop` profile, state persisted | `profile-desktop` |
| **18. Init**: commented example host read top to bottom, `nix run .#init` writes a fork's first host and drops the examples, every example host commented | `init`, `example-hosts` |
| **19. Hibernate**: opt-in `hibernate`, swap file in LUKS on its own subvolume, resume through systemd before the `/` rollback, suspend-then-hibernate | `feature-hibernate` |
| **20. Doctor**: `doctor` on every machine, Secure Boot, TPM2, snapshots, failed units, updates, and the files the next boot erases | `base-doctor`, `<host>-disk` |
| **21. CI report**: one report per run in the run summary and a single PR comment, a table per machine saying what each check proves, the likely cause of a failure and what to do, a flaky VM test retried once and marked unstable | `ci-report` |
| **22. Registry layout**: `modules/` reads as the registry (`features/`, `apps/`, `users/`, `hosts/`, `flake/`), one apps layer for everything a user runs, people defined once in `modules/users/<name>` with an account per host, `init` writes the person too | `wrong-namespace`, `apps-once`, `flake-modules`, `example-hosts`, `init` |
| **23. Wiped home by default**: `example` turns `wipe-home` on, every home keeps `~/.ssh`, `~/.gnupg` and `~/.config/gh` (mode `0700`), CUPS, power-profiles-daemon and fwupd state start fresh at boot | `example-disk`, `example-server-disk`, `profile-desktop` |
| **24. Validated upgrades**: CI moves `deploy/<host>` once every check of that host passed, `auto-upgrade` applies only that version at boot from the fork or a mirror kept up to date by CI, and a generation that does not start is rolled back | `ci-promote`, `feature-auto-upgrade`, `feature-rollback`, `feature-rollback-secure-boot`, `ci-mirror` |
| **25. Upstream**: `nix run .#upstream` shows a fork the starter's breaking changes and changelog since its last merge, then prepares the merge branch and its PR; the weekly `sync-upstream` runs only when the fork sets `SYNC_UPSTREAM` | `upstream` |
| **26. Base defaults**: journal capped at 500 MB on every host, AppImages run by their name with `nix-ld`, CPU microcode with `firmware` | `base-journald`, `feature-nix-ld`, `profile-pi`, `example-hosts` |
| **27. Upgrade feedback**: `doctor` says what the last auto-upgrade did, warns when it failed, stalled or a boot went back to the previous version, `doctor --history` lists the last runs, and a desktop shows a notification once per upgrade | `app-upgrade-notify`, `base-doctor` |
| **28. Try the install**: `nix run .#try-install -- <host>` runs a host's installer ISO in a local QEMU window with Secure Boot in setup mode, a software TPM2 and a blank disk image kept between runs, and refuses any device | `try-install` |
| **29. Update path proofs**: boot counting proven on the locked systemd and lanzaboote with a health check that fails before `boot-complete.target`, the `auto-upgrade` check of units failing after boot kept, and a machine running the `example` host of the base revision keeps its state after switching to the new version | `feature-rollback`, `feature-rollback-secure-boot`, `feature-rollback-failed-unit`, `feature-migration`, `feature-migration-without-disk` |
| **30. CI evidence**: the CI report shows each machine's `systemd-analyze` boot time beside the base revision's and links the screenshots VM tests take as run artifacts; a nightly `reproducibility` run rebuilds every system and summarizes differences with diffoscope, never required by `ci-ok`; aarch64 VM tests stay on `x86_64-linux` since GitHub's arm64 runners have no KVM | `ci-report`, `reproducibility`, `example-boots`, `example-server-boots` |
| **31. Signed deploy refs**: CI signs each promotion of `deploy/<host>` with an SSH key kept in the `deploy` environment, as a commit holding the previous promotion and the validated commit of `main`; a host with `signers` applies only a commit signed for it, skips a source that serves anything else and ends `refused` when none does; `init` generates the key pair, `doctor` warns on a host without `signers` | `ci-promote`, `feature-auto-upgrade-signed`, `base-doctor`, `init` |
