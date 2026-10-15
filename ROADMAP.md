# Roadmap

From the deepest layer to the most visible one: each lot builds and boots on its own before the next starts. A shipped lot keeps a guardrail in CI; a change that touches it keeps that guardrail green.

Everything is developed and tested in VMs. Installing on real hardware is each fork's business, not a lot of this starter.

## Shipped

| Lot | Guardrail |
|---|---|
| **1. Repo tooling**: dendritic flake, nixfmt + statix + deadnix, build check per host, boot test, CI matrix with KVM, PR package diff, weekly lock update merged on green CI, `develop` sync, release PRs with release-please, MIT license | `nix flake check` (`treefmt`, `<host>-toplevel`, `desktop-boots`), `ci-ok` required on `main` |

## Next

### 2. Generic starter

- Rename the example host to `example`; `desktop` and `server` become profiles
- Host registry (`configurations.nixos.<name>`): declaring a host generates its `nixosConfigurations` entry, build check and boot test
- CI matrix carries system and runner, ready for `aarch64-linux`
- Starter modules expose options with neutral defaults; personal values (locale, keyboard, time zone) move to hosts and users
- CONTRIBUTING, SECURITY, CODE_OF_CONDUCT

Guardrail: every declared host gets a build check and a boot test without extra code.

### 3. Disk

disko layout, LUKS unlocked by TPM2 (passphrase fallback), btrfs subvolumes, impermanence of `/` with `/home` and `/persist` kept.

Guardrail: VM test that reboots and checks `/` is wiped while declared state survives.

### 4. Boot

Kernel choice, systemd initrd, Secure Boot with lanzaboote, optional Windows entry in the boot menu.

Guardrail: VM test booting under OVMF with Secure Boot enforced.

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

`aarch64-linux` host on the `pi` profile through `nvmd/nixos-raspberrypi` (Pi 5 has no official nixpkgs support), SD image output, CI on `ubuntu-24.04-arm`.

Guardrail: SD image builds in CI.
