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

## Next

### 5. Hardware

Firmware and microcode, PipeWire, Bluetooth, nixos-hardware profiles. Opt-in GPU modules: NVIDIA (open kernel module), AMD, Intel.

Guardrail: host builds per hardware profile, and the NVIDIA kernel module builds against the kernel each profile ships.

### 6. Performance profiles

`desktop`, `laptop`, `server` and `pi` profiles, one power manager per profile, only settings backed by a measured gain:

- Kernel: latest on desktop and laptop, stable on servers; CachyOS kernel (`xddxdd/nix-cachyos-kernel`) evaluated against latest before adoption
- sched-ext (`services.scx`): `scx_lavd` or `scx_bpfland` on desktop, benchmarked on the machine; off on servers and Pi
- Power: power-profiles-daemon on desktop and laptop, thermald on Intel laptops, tuned on servers
- Memory and IO: zram with zstd (never with zswap), tmpfs `/tmp` on desktop, btrfs `compress=zstd` and `noatime`, monthly scrub
- Nix: parallel builds, automatic garbage collection, Pi builds offloaded to a remote builder
- Excluded: `mitigations=off`, `-march=native` on nixpkgs

Guardrail: VM test asserting the active scheduler, governor and zram per profile.

### 7. Services

resolved, SSH, secrets with sops-nix, btrfs snapshots. Opt-in: Tailscale.

Guardrail: boot test checks the services are active and a secret decrypts.

### 8. Dev base

nh, nix-index-database, Nix garbage collection. Opt-in: Docker, libvirt. Each module declares its own state in `/persist` (`/var/lib/docker`, `/var/lib/libvirt`).

### 9. Shell and tools

zsh, git, nvim, terminal, CLI tools through home-manager. Opt-in: wipe `/home` at boot like `/`, keeping only the paths each user declares (`environment.persistence."/persist".users.<user>`): projects, keys, app config, sessions and state. A guide explains how to list what an existing home needs to keep.

### 10. Desktop

greetd and Plymouth, then one desktop module as the example: Hyprland with UWSM, bar, launcher, theme. Other desktops plug in as sibling modules.

Guardrail: VM test reaching the graphical session.

### 11. Raspberry Pi

`aarch64-linux` host on the `pi` profile through `nvmd/nixos-raspberrypi` (Pi 5 has no official nixpkgs support), SD image output, CI on `ubuntu-24.04-arm`. Check that `ubuntu-24.04-arm` exposes `/dev/kvm` for the boot test (otherwise it runs under slow TCG emulation), and extend the PR package diff to `aarch64-linux` hosts. The `disk` module assumes UEFI with systemd-boot: a Pi gets its own disk layout or none.

Guardrail: SD image builds in CI.
