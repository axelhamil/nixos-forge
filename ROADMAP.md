# Roadmap

From the deepest layer to the most visible one: each lot builds and boots on its own before the next starts. A shipped lot keeps a guardrail in CI; a change that touches it keeps that guardrail green.

Everything is developed and tested in VMs. Installing on real hardware is each fork's business, not a lot of this starter.

## Shipped

| Lot | Guardrail |
|---|---|
| **1. Repo tooling**: dendritic flake, nixfmt + statix + deadnix, build check per host, boot test, CI matrix with KVM, PR package diff, weekly lock update merged on green CI, `develop` sync, release PRs with release-please, MIT license | `nix flake check` (`treefmt`, `<host>-toplevel`, `<host>-boots`), `ci-ok` required on `main` |
| **2. Generic starter**: host registry `configurations.nixos.<name>` generating the config, build check and boot test, CI matrix per system and runner (`aarch64-linux` ready), neutral `example` host and `alice` user, locale in the host, `desktop` profile, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT | every registry entry gets `<name>-toplevel`, `<name>-boots` and a matrix entry on its system's runner (`nix eval --json .#ci.matrix`) |
| **3. Disk**: disko layout, LUKS2 with TPM2 unlock enrolled after install (passphrase fallback), btrfs subvolumes `@root`, `@nix`, `@persist`, `@home` with zstd, `/` restored from a blank snapshot in the systemd initrd, impermanence list, immutable users, install guide, `/persist` declared per module | `<host>-disk`: installs on a blank disk, reboots, checks `/` is wiped while `/persist`, `/home` and `machine-id` survive |

## Next

### 4. Boot

Kernel choice, systemd initrd, Secure Boot with lanzaboote, optional Windows entry in the boot menu.

Guardrail: VM test booting under OVMF with Secure Boot enforced, and the disk unlocked by an emulated TPM (swtpm) with no key file.

### 5. Hardware

NVIDIA (open kernel module), firmware and microcode, PipeWire, Bluetooth, nixos-hardware profiles.

Guardrail: host builds per hardware profile.

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

NetworkManager and resolved, Tailscale, SSH, secrets with sops-nix, btrfs snapshots.

Guardrail: boot test checks the services are active and a secret decrypts.

### 8. Dev base

Docker, libvirt, nh, nix-index-database, Nix garbage collection.

### 9. Shell and tools

zsh, git, nvim, terminal, CLI tools through home-manager.

### 10. Desktop

greetd, Hyprland with UWSM, bar, launcher, theme.

Guardrail: VM test reaching the graphical session.

### 11. Raspberry Pi

`aarch64-linux` host on the `pi` profile through `nvmd/nixos-raspberrypi` (Pi 5 has no official nixpkgs support), SD image output, CI on `ubuntu-24.04-arm`. Check that `ubuntu-24.04-arm` exposes `/dev/kvm` for the boot test (otherwise it runs under slow TCG emulation), and extend the PR package diff to `aarch64-linux` hosts. The `disk` module assumes UEFI with systemd-boot (its install test passes the LUKS key through `boot.initrd.secrets`, which extlinux rejects): a Pi gets its own disk layout or none.

Guardrail: SD image builds in CI.
