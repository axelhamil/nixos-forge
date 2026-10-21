# Doctor

Ask a machine how it is doing. Every host has a `doctor` command. It checks the boot chain, the disk, the updates and what the next boot will erase, then says what to do about each problem.

It prints one line per check: `OK`, `WARN`, `FAIL`, or `SKIP` when the host lacks the feature, with a hint under each problem. Run it as `sudo doctor`; without root it skips the checks that need it and says so. It exits non-zero when a check fails.

```text
[  OK] Secure Boot    enforced, keys enrolled
[  OK] LUKS unlock    TPM2 token, passphrase kept as a fallback
[  OK] Snapshots      last btrbk run 12 min ago, scheduled every 1 h
[  OK] Disk space     81 % free on / (btrfs, 380G left)
[  OK] Failed units   none
[WARN] Updates        /home/<you>/nixos: behind by 3
                      run update
[  OK] Upgrades       upgraded 2026-05-12 04:21: Closure size: 2154 -> 2161 (31 paths added, 24 paths removed, delta +12.4 MiB)
[  OK] Signed deploys auto-upgrade applies only deploy commits signed by 1 key of update.signers
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
| Signed deploys | `auto-upgrade` | the host's `signers`: a warning, at every run, while it has none and `auto-upgrade` applies any deploy commit its sources serve ([updates.md](../explanation/updates.md#signed-deploy-commits)) |
| Boot | systemd-boot, as root | `bootctl list`: a warning when the newest generation spent its tries (`+0-`) and the machine runs an older one: it went back to the previous version |
| Impermanence | `disk` | files on `/` written since `sysinit.target`, which the rollback erases |

## Upgrade history

`doctor --history` prints the last 10 runs of `auto-upgrade` from `/var/lib/auto-upgrade/history.tsv`, without running the checks:

```text
TIME                       OUTCOME      FROM          TO            SOURCE
2026-05-11T04:12:09+02:00  up-to-date   3f9c2a1b7d40  3f9c2a1b7d40  https://github.com/<owner>/<repo>.git
2026-05-12T04:21:44+02:00  upgraded     3f9c2a1b7d40  8e01d4c9a2b7  https://github.com/<owner>/<repo>.git
```

## What the next boot erases

With `disk`, `/` is restored from an empty snapshot at every boot. This is impermanence: only `/nix`, `/home`, `/persist`, `/boot` and the paths you persist survive. This section is the home page for its rules.

Whatever NixOS creates during the boot (`/etc`, users, tmpfiles) comes back on its own. So `doctor` lists only files created or changed on `/` after `sysinit.target`, outside any mount. It leaves out:

- persisted paths, which are bind mounts from `/persist`;
- `/nix`, `/home`, `/persist` and `/boot`, which are their own subvolumes;
- symlinks into `/etc/static` or the store;
- the files activation rewrites (`/etc/passwd`, the files in `/etc/.clean`), so a `nixos-rebuild switch` adds nothing.

Each line is a choice, made in the module that owns the file, never in a central list:

- keep it with `environment.persistence."/persist"`;
- or declare its loss expected with `doctor.ephemeralPaths`.

State in a home follows the same rule on a host with `wipe-home` ([wipe-home](wipe-home.md)).

The shipped modules already list their own lost files:

| Module | Expected losses |
|---|---|
| `base` | `/tmp`, `/var/tmp`, `/var/lib/lastlog` |
| `server` | tuned's runtime profile |
| `desktop` | NetworkManager's unused `wpa_supplicant` file, power-profiles-daemon's state |
| `printing` | the CUPS state |
| `firmware` | fwupd's state |
| `plymouth` | Plymouth's boot timing |

A fresh boot of upstream's examples reports nothing.
