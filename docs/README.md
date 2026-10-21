# Docs

Every page of `docs/`, linked from here; the `docs` check fails on a page this index does not link.

New here? Read [the README](../README.md) first, then [Install on a machine](tutorials/install.md). Come back to the how-to guides when you want one thing, and to the reference when you look something up.

## Tutorials

- [Install on a machine](tutorials/install.md): from a host file to a real machine that boots, unlocks with the TPM2 and rolls back a bad upgrade, one checked step at a time.

## How-to guides

- [Install without the ISO](how-to/install-without-iso.md): over SSH with nixos-anywhere, or by hand with disko-install.
- [Start `/home` empty at every boot](how-to/wipe-home.md): a home that forgets what you did not declare.
- [Hibernate](how-to/hibernate.md): resume the same session later.
- [FIDO2](how-to/fido2.md): log in, `sudo` and unlock the disk with a security key.
- [Dual boot with Windows](how-to/dual-boot.md): Windows in the systemd-boot menu, under Secure Boot.
- [Raspberry Pi](how-to/raspberry-pi.md): from a host to a flashed card, kept up to date.
- [Secrets](how-to/secrets.md): encrypted passwords and tokens in your fork, decrypted by each machine at boot.
- [Variants](how-to/specialisations.md): several setups of one machine, picked at boot.
- [Doctor](how-to/doctor.md): ask a machine how it is doing.
- [CI and repository settings](how-to/ci.md): the settings a fork needs, the report, deploy branches and mirrors.

## Reference

- [Host options](reference/options.md): every field of a host and of a person.
- [Features](reference/features.md): the disk layout, Secure Boot and unattended upgrades.
- [Hardware](reference/hardware.md): what each hardware module turns on.
- [Apps](reference/apps.md): everything a user runs, with its config in Nix.
- [Desktop](reference/desktop.md): a login screen, a boot splash and Hyprland.
- [Dev base](reference/dev.md): Nix housekeeping, containers, VMs and per-project shells.
- [Services](reference/services.md): DNS, SSH, secrets, printing and snapshots.
- [Performance](reference/performance.md): what each profile brings and tunes.
- [CI](reference/ci.md): the workflows, every check and the CI scripts you can run locally.

## Explanation

- [Architecture](explanation/architecture.md): the forge rule and how a host entry becomes a system.
- [Updates](explanation/updates.md): how a change reaches your machines without skipping CI.

## Maintaining the starter <!-- docs-check: allow the starter -->

Pages for the maintainer of the upstream repository; a fork needs none of them.

- [Releases](maintenance/releases.md): branches, release-please and where forks find upstream.
- [CI of the examples](maintenance/ci.md): the checks of the examples and of the upgrade path.
