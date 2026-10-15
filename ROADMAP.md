# Roadmap

From the deepest layer to the most visible one: each lot builds and boots on its own before the next starts. A shipped lot keeps a guardrail in CI; a change that touches it keeps that guardrail green.

The starter is universal: a fork picks what it needs. Anything beyond the shared base (disk, boot, Nix) ships as an opt-in module a host imports, with neutral defaults; personal choices belong in forks, not here.

Everything is developed and tested in VMs. Installing on real hardware is each fork's business, not a lot of this starter.

## Shipped

| Lot | Guardrail |
|---|---|
| **1. Repo tooling**: dendritic flake, nixfmt + statix + deadnix, build check per host, boot test, CI matrix with KVM, PR package diff, weekly lock update merged on green CI, `develop` sync, release PRs with release-please, MIT license | `nix flake check` (`treefmt`, `<host>-toplevel`, `<host>-boots`), `ci-ok` required on `main` |
| **2. Generic starter**: host registry `configurations.nixos.<name>` generating the config, build check and boot test, CI matrix per system and runner (`aarch64-linux` ready), neutral `example` host and `alice` user, locale in the host, `desktop` profile, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT | every registry entry gets `<name>-toplevel`, `<name>-boots` and a matrix entry on its system's runner (`nix eval --json .#ci.matrix`) |
| **3. Disk**: disko layout, LUKS2 with TPM2 unlock enrolled after install (passphrase fallback), btrfs subvolumes `@root`, `@nix`, `@persist`, `@home` with zstd, `/` restored from a blank snapshot in the systemd initrd, impermanence list, immutable users, install guide, `/persist` declared per module | `<host>-disk`: installs on a blank disk, reboots, checks `/` is wiped while `/persist`, `/home` and `machine-id` survive |
| **4. Boot**: opt-in `secure-boot` module with lanzaboote, keys generated on the machine and enrolled from setup mode (Microsoft keys kept), TPM2 unlock bound to PCR 7 and to PCR 15 at zero (volume key measured by the initrd) once Secure Boot is enforced | `<host>-disk` on an `x86_64-linux` `secure-boot` host: boots the installed disk under OVMF in setup mode with swtpm, checks Secure Boot is enforced and the disk unlocks with the TPM alone, passphrase removed |
| **5. Hardware**: opt-in `firmware` (redistributable firmware, fwupd signed by lanzaboote), `audio` (PipeWire), `bluetooth`, `nvidia` (open kernel module), `amd-gpu`, `intel-gpu` modules, nixos-hardware input, unfree packages allowed by name (`allowUnfreePackages`) instead of a global `allowUnfree` | `<host>-nvidia` on every `x86_64-linux` host without NVIDIA: the host with the `nvidia` module evaluates and the NVIDIA open kernel module builds against its kernel, so a lock update that breaks it never merges |
| **6. Performance profiles**: `desktop` (latest kernel, sched-ext `scx_lavd`, power-profiles-daemon, systemd-oomd, tmpfs `/tmp`), `laptop` (`scx_lavd --autopower`), `server` (LTS kernel, tuned); zram with zstd, weekly `nix.gc` (replaced by `nh clean` in lot 8), store optimisation timer, monthly btrfs scrub; `nix run .#bench` (schbench packaged in the flake, perf, hackbench, stress-ng, fio) to measure on real hardware (CachyOS kernel, thermald and zram sysctls wait for a measurement) | `profile-desktop`, `profile-laptop`, `profile-server`: VM tests asserting the active scheduler, power service, zram, tmpfs `/tmp` and oomd per profile |
| **7. Services**: systemd-resolved in every host (no compiled-in fallback DNS, LLMNR off), btrbk snapshots of `@home` and `@persist` into a new `@snapshots` subvolume (hourly, 2 days, 14 daily, 4 weekly), opt-in `ssh` (public keys only, no root, single ED25519 host key in `/persist`, post-quantum key exchange, finite-field Diffie-Hellman refused), opt-in `secrets` (sops-nix with a per-machine post-quantum age key in `/persist`), `sops` and `age` in `nix develop`; no service depends on a third-party cloud | `system-resolved`, `system-ssh` (key login from a second VM, password and root refused, `mlkem768x25519-sha256` negotiated), `system-secrets` (a fixture decrypts to `/run/secrets`); `<host>-disk` takes a snapshot and checks it sits in its own subvolume, outside the wiped `@root` |
| **8. Dev base**: `nh clean` (14 days and the last 5 generations, latest dev shell per direnv project, weekly, persistent) replaces `nix.gc`, channels off, nix-index-database (small database) with comma and command-not-found offline, opt-in rootless `docker` (containerd image store, `local` logs, persistent weekly prune, cgroup controllers delegated), opt-in `libvirt` (unprivileged QEMU, swtpm, virtiofs, USB redirection, virt-manager, `nodatacow` images subvolume kept out of snapshots, state in `/persist`), opt-in `nix-ld` and `direnv` (nix-direnv) | `system-nix` (nh timer, no `nix.gc`), `system-nix-index` (`nix-locate` offline, comma), `dev-docker` (rootless container on a btrfs `/home`, containerd `overlayfs` snapshotter, `local` logs, persistent prune timer), `dev-libvirt` (libvirtd answers, `nodatacow` subvolume on btrfs, emulated TPM) |

## Next

### 9. Shell and tools

zsh, git, nvim, terminal, CLI tools through home-manager. Opt-in: wipe `/home` at boot like `/`, keeping only the paths each user declares (`environment.persistence."/persist".users.<user>`): projects, keys, app config, sessions and state (rootless Docker's `~/.local/share/docker` included). A guide explains how to list what an existing home needs to keep.

### 10. Desktop

greetd and Plymouth, then one desktop module as the example: Hyprland with UWSM, bar, launcher, theme. Other desktops plug in as sibling modules.

Guardrail: VM test reaching the graphical session.

### 11. Raspberry Pi

A `pi` profile (stable kernel, no sched-ext, builds offloaded to a remote builder with `nix.buildMachines`) and an `aarch64-linux` host on it through `nvmd/nixos-raspberrypi` (Pi 5 has no official nixpkgs support), SD image output, CI on `ubuntu-24.04-arm`. Check that `ubuntu-24.04-arm` exposes `/dev/kvm` for the boot test (otherwise it runs under slow TCG emulation), and extend the PR package diff to `aarch64-linux` hosts. The `disk` module assumes UEFI with systemd-boot: a Pi gets its own disk layout or none.

Guardrail: SD image builds in CI.

### 12. Simplify for a newcomer

Last pass over the whole starter, once every lot is in: someone who has never used Nix forks it and gets a working machine by editing one place. A single host file with the choices a newcomer makes (user, hostname, locale, disk, profile, opt-in modules as plain switches), sensible defaults for everything else, no Nix knowledge needed for the common path. Merge or drop modules and options that do not earn their place, shorten the docs to what a first install needs, and keep the advanced paths documented separately.

Guardrail: a fresh fork configured only through that file builds, boots and passes `<host>-disk` in CI.
