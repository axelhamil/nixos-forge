# Updates

How a change reaches your machines: every Monday your fork gets PRs for new inputs and for the starter's latest release, its CI proves them, then each machine pulls your fork.

```text
nixos-starter release ──┐
nixpkgs and inputs ─────┤ Monday PRs: inputs merge once ci-ok is green,
                        │ the starter's release waits for your review
                        ▼
                your fork's main ──▶ `update` on a desktop, `auto-upgrade` on a server
```

## Your fork follows the starter

The `sync-upstream` workflow runs every Monday at 07:00 UTC, or by hand, in your fork only. When the starter has a release your fork does not hold, it merges that release tag into a `chore/sync-upstream-<tag>` branch and opens a PR against `main`, listing any breaking change (a `!` commit or a `BREAKING CHANGE` footer) since your last sync.

It never merges on its own: the starter's code reaches your machines only once you have read the diff. Check the [release notes](https://github.com/axelhamil/nixos-starter/releases), adapt your hosts on the branch if needed, wait for `ci-ok`, then merge with a merge commit, never a squash, so the next sync starts from there. On a conflict the job fails and names the files: merge the tag by hand as its error says, then push.

The starter keeps `modules/hosts` and `modules/users` minimal, so conflicts stay rare as long as your own config lives there. Only the starter publishes releases (`release` is skipped in forks), so `CHANGELOG.md` and the version files never conflict.

`update-flake-lock` runs one hour earlier and opens a PR updating `flake.lock`, which merges itself once `ci-ok` is green: it only moves inputs (nixpkgs, home-manager, ...) to newer revisions of the same sources. Both workflows need the bot App and the settings of [ci.md](ci.md#turn-ci-on-in-your-fork).

## A machine follows your fork

Tell the host where it updates from, in its registry entry:

```nix
configurations.nixos.my-laptop.flake = "/home/alice/nixos";
```

**A checkout on the machine** (desktops, laptops): run `update`. It pulls your fork with `git pull --ff-only`, refusing a checkout that has diverged, then runs `nh os switch`, which builds, shows the package diff and switches. Extra arguments go to nh (`update --ask`). `nh os switch` alone works too, since `flake` is also its default.

**A flake reference** (servers, Raspberry Pi): set `flake = "github:alice/nixos"` and add the `auto-upgrade` feature. Every Tuesday from 04:00 (randomized over 45 minutes), after Monday's PRs and their CI, the machine builds your fork's `main` and switches to it, so it runs exactly what CI proved. It does not reboot: a new kernel waits for the next reboot. `update` works there too, with no checkout to pull.
