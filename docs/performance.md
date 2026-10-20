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

`pi` extends `server` and adds a 15 s hardware watchdog and a 100 MB journal cap instead of 500 MB ([raspberry-pi.md](raspberry-pi.md)).

Every host also gets the journal capped at 500 MB (with `disk`, `/var/log` is kept in `/persist`, where systemd would let it grow to 10% of the file system, at most 4 GB; set `services.journald.settings.Journal.SystemMaxUse` to change it), a weekly `nh clean` ([dev.md](dev.md)), automatic store optimisation (`nix.optimise.automatic`, which a host may turn off), and, with `disk`, a monthly btrfs scrub.

systemd-oomd kills the cgroup under the most memory pressure once a slice stays above 80%; it does not act on swap usage. `nix-daemon` has `ManagedOOMPreference=avoid`, so a big build is not the first victim. Nix builds in `/nix/var/nix/builds`, not in `/tmp`.

