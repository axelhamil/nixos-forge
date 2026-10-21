# Features

The features that decide how a machine boots, stores its data and upgrades. A host lists them in `features`. Each section says what it turns on and links to the page that goes through it.

The other features have their own page or section: `wipe-home`, `hibernate`, `fido2`, `dual-boot`, `greetd`, `plymouth`, `ssh`, `secrets`, `printing`, `docker`, `libvirt` and `nix-ld` ([docs index](../README.md)).

## `disk`

The encrypted disk layout and the wiped `/`.

> [!WARNING]
> Installing a host with `disk` erases the whole disk it is given ([install.md](../tutorials/install.md)).

- A GPT disk: a 1 GiB ESP on `/boot`, then LUKS2 `cryptroot` holding btrfs subvolumes for `/`, `/nix`, `/persist`, `/home` and `/.snapshots` ([the layout](../tutorials/install.md#what-the-disk-looks-like)).
- `/` restored from an empty snapshot at every boot, with impermanence keeping what modules declare in `/persist` ([the rules](../how-to/doctor.md#what-the-next-boot-erases)).
- Users immutable, their passwords read from `/persist/passwords/<user>`.
- The TPM2 tried first to open the disk (`tpm2-device=auto`), the passphrase as fallback.
- systemd-boot with its editor off and 3 tries for each new generation ([going back](../tutorials/install.md#going-back-to-the-previous-generation)), and a monthly btrfs scrub.
- The options `disk.device` (the disk erased at install, `/dev/null/no-disk-chosen` by default so a forgotten device erases nothing), `disk.passphraseFile` and `disk.unlockTestScript`.

## `secure-boot`

Secure Boot through [lanzaboote](https://github.com/nix-community/lanzaboote), in place of plain systemd-boot.

> [!WARNING]
> The keys are enrolled alongside Microsoft's, which GPU option ROMs and Windows need: never enroll your own keys alone ([setup mode](../tutorials/install.md#step-2-put-the-firmware-in-setup-mode)).

- Keys generated at the first boot in `/var/lib/sbctl`, kept in `/persist`, and enrolled from the firmware's setup mode.
- 3 tries for each new generation, as with `disk`.
- `sbctl` installed, and with `disk` the TPM2 enrolled at the first boot under Secure Boot ([step 6](../tutorials/install.md#step-6-check-the-tpm2-unlocks-the-disk)).

## `auto-upgrade`

Daily upgrades to the version CI validated for the host. The flow, its sources, outcomes and signed deploys are in [updates.md](../explanation/updates.md#a-machine-upgrades-on-its-own).

- Every day from 04:00, randomized over 45 minutes; the new version applies at the next boot, and a version that does not start is abandoned after 3 tries on a host with `disk`.
- The last run and its history in `/var/lib/auto-upgrade`, kept in `/persist`, read by `doctor` and `upgrade-notify`.
