# doctor

Every machine has a `doctor` command: one line per check, `OK`, `WARN`, `FAIL`, or `SKIP` when the host lacks the feature, with a hint under each problem. Run it as `sudo doctor`: without root it skips the checks that need it and says so. It exits non-zero when a check fails.

```
[  OK] Secure Boot    enforced, keys enrolled
[  OK] LUKS unlock    TPM2 token, passphrase kept as a fallback
[  OK] Snapshots      last btrbk run 12 min ago, scheduled every 1 h
[  OK] Disk space     81 % free on / (btrfs, 380G left)
[  OK] Failed units   none
[WARN] Updates        /home/alice/nixos: behind by 3
                      run update
[  OK] Upgrades       upgraded 2026-05-12 04:21: Closure size: 2154 -> 2161 (31 paths added, 24 paths removed, delta +12.4 MiB)
[  OK] Boot           the newest generation started, or has tries left
[WARN] Impermanence   2 files written to / since boot are lost at the next boot
                      keep them in environment.persistence."/persist", or list them in doctor.ephemeralPaths
                          1  /srv/app
                          1  /
                      doctor --all lists every file
```

| Check | Runs when | Looks at |
|---|---|---|
| Secure Boot | `secure-boot` | the `SecureBoot` and `SetupMode` EFI variables: enforced with enrolled keys, or why not |
| LUKS unlock | `disk` and `secure-boot`, as root | the keyslots of `disk-main-luks`: a TPM2 token, and a passphrase left to fall back on |
| Snapshots | `disk` | btrbk's last run against its schedule (a warning past twice the interval), and a snapshot of each subvolume |
| Disk space | always | free space on `/`, a warning under 10 % |
| Failed units | always | failed system units and the calling user's |
| Updates | `flake` is a git checkout on the machine | fetches as the checkout's owner, then behind, ahead and uncommitted changes |
| Upgrades | `auto-upgrade` | the last run in `/var/lib/auto-upgrade/state.json`: when it upgraded and what changed, or since when the machine is up to date; a warning with the reason when it was `refused`, `unreachable`, `waiting`, `failed` or `rolled-back`, or when the last success is more than 14 days old |
| Boot | systemd-boot, as root | `bootctl list`: a warning when the newest generation spent its tries (`+0-`) and the machine runs an older one: it went back to the previous version |
| Impermanence | `disk` | files on `/` written since `sysinit.target`, which the rollback erases |

## Upgrade history

`doctor --history` prints the last 10 runs of `auto-upgrade` from `/var/lib/auto-upgrade/history.tsv`, without running the checks:

```
TIME                       OUTCOME      FROM          TO            SOURCE
2026-05-11T04:12:09+02:00  up-to-date   3f9c2a1b7d40  3f9c2a1b7d40  https://github.com/alice/nixos.git
2026-05-12T04:21:44+02:00  upgraded     3f9c2a1b7d40  8e01d4c9a2b7  https://github.com/alice/nixos.git
```

## What the next boot erases

With `disk`, `/` is restored from an empty snapshot at every boot. Whatever NixOS creates during the boot (`/etc`, users, tmpfiles) comes back on its own, so `doctor` only lists files created or changed on `/` after `sysinit.target` was reached, outside any mount: persisted paths are bind mounts from `/persist`, and `/nix`, `/home`, `/persist`, `/boot` are their own subvolumes, so none of them shows up. Symlinks into `/etc/static` or the store and the files activation rewrites (`/etc/passwd`, the files in `/etc/.clean`) are left out, so a `nixos-rebuild switch` adds nothing to the list.

Each line is a choice: keep the file with `environment.persistence."/persist"` in the module that owns it, or declare its loss expected with `doctor.ephemeralPaths`, again from the module that owns it. The starter lists `/tmp`, `/var/tmp` and `/var/lib/lastlog` in `base`, tuned's runtime profile in `server`, NetworkManager's unused `wpa_supplicant` file and the CUPS and power-profiles-daemon state in `desktop`, fwupd's state in `firmware` and Plymouth's boot timing in `plymouth`; a fresh boot of the example hosts reports nothing.
