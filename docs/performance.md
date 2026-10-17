# Performance

What each profile tunes.

## Profiles

| | `desktop` | `laptop` | `server` | `pi` |
|---|---|---|---|---|
| Kernel | latest | latest | default (LTS) | vendor kernel from the board module |
| CPU scheduler | sched-ext `scx_lavd` | `scx_lavd --autopower` (follows the power profile) | kernel default (EEVDF) | kernel default |
| Power | power-profiles-daemon | power-profiles-daemon | tuned | tuned |
| Out of memory | systemd-oomd on root and user slices, `nix-daemon` spared | same | kernel default | kernel default |
| `/tmp` | tmpfs | tmpfs | on disk | on disk |
| Printing | CUPS, driverless (IPP Everywhere), network printers found through Avahi | same | none | none |

`pi` extends `server` and adds a 15 s hardware watchdog and a 100 MB journal cap ([raspberry-pi.md](raspberry-pi.md)).

Every host also gets zram swap, a weekly `nh clean` ([dev.md](dev.md)), automatic store optimisation, and, with `disk`, a monthly btrfs scrub.

systemd-oomd kills the cgroup under the most memory pressure once a slice stays above 80%; it does not act on swap usage. `nix-daemon` has `ManagedOOMPreference=avoid`, so a big build is not the first victim. Nix builds in `/nix/var/nix/builds`, not in `/tmp`.

