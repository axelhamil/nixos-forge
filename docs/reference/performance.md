# Performance

What each profile tunes, and what every host gets anyway. Pick the role in the host's `profile`; nothing here needs a setting.

A profile brings only what its role needs: firmware, audio and Bluetooth are hardware modules (`firmware`, `audio`, `bluetooth`) and printing is the `printing` feature ([services.md](services.md#printing-printing-opt-in)), each listed in the host.

## Profiles

| | `desktop` | `laptop` | `server` | `pi` |
|---|---|---|---|---|
| Kernel | latest | latest | default (LTS) | vendor kernel from the board module |
| CPU scheduler | sched-ext `scx_lavd` | `scx_lavd --autopower` (follows the power profile) | kernel default (EEVDF) | kernel default |
| Power | power-profiles-daemon | power-profiles-daemon | tuned | tuned |
| Out of memory | systemd-oomd on root and user slices, `nix-daemon` spared | same | kernel default | kernel default |
| `/tmp` | tmpfs | tmpfs | on disk | on disk |
| Swap | zram | zram | none | zram |

### `desktop`

For a machine you sit at. NetworkManager, with its connections kept in `/persist`; the latest kernel (a `mkDefault`); the `scx_lavd` scheduler; power-profiles-daemon; systemd-oomd on the root and user slices, with `nix-daemon` spared; `/tmp` on tmpfs; zram swap.

### `laptop`

The `desktop` profile, with `scx_lavd --autopower`, so the scheduler follows the power profile.

### `server`

For a machine nobody sits at. tuned, without its power-profiles-daemon support, and the kernel defaults for everything else.

### `pi`

For a Raspberry Pi ([raspberry-pi.md](../how-to/raspberry-pi.md)). The `server` profile, plus zram swap, a 15 s hardware watchdog, a 100 MB journal cap instead of 500 MB, no NixOS manual and passwordless `sudo` for `wheel`. Builds go to `nix.buildMachines` when the host lists any, and those substitute from the binary cache themselves. `auto-upgrade` never builds locally. Each choice but the journal cap and zram is a `mkDefault`.

Every host also gets:

- the journal capped at 500 MB. With `disk`, `/var/log` is kept in `/persist`, where systemd would let it grow to 10% of the file system, at most 4 GB. Set `services.journald.settings.Journal.SystemMaxUse` to change it;
- a weekly `nh clean` ([dev.md](dev.md));
- automatic store optimisation (`nix.optimise.automatic`, which a host may turn off);
- with `disk`, a monthly btrfs scrub.

systemd-oomd kills the cgroup under the most memory pressure once a slice stays above 80%; it does not act on swap usage. `nix-daemon` has `ManagedOOMPreference=avoid`, so a big build is not the first victim. Nix builds in `/nix/var/nix/builds`, not in `/tmp`.

