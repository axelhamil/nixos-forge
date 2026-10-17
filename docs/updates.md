# Updates

How a change reaches your machines: every Monday your fork gets a PR for new inputs, you merge the starter's releases when you choose, CI proves both, then each machine pulls your fork.

```text
nixos-starter release ──┐ `nix run .#upstream`: a branch and a PR
nixpkgs and inputs ─────┤ for your review; Monday PR: inputs merge
                        │ once ci-ok is green
                        ▼
                your fork's main ──▶ `update` on a desktop
                        │ CI passes for a host
                        ▼
                deploy/<host> ───▶ `auto-upgrade` on that host
```

## Your fork follows the starter

Your fork decides when it takes the starter. In your checkout, with no uncommitted change and on any branch, run:

```console
$ nix run .#upstream
```

It reads the starter's latest release tag (`git ls-remote`, no GitHub login needed) and, when the `main` branch of your `origin` does not hold it yet, prints what changed since your last merge: the breaking changes first (a `!` commit or a `BREAKING CHANGE` footer), then the new lines of the starter's `CHANGELOG.md`. It then asks `Prepare branch chore/sync-upstream-<tag>? [y/N]`; on yes it creates that branch from `origin/main` (never from the branch you are on, so unreleased work on `develop` stays out of it), merges the tag with a merge commit, pushes it to `origin` and opens a PR against `main` of `origin` with `gh` when it is logged in, or prints the `gh pr create --repo <owner>/<repo>` command to run. `--yes` skips the question. The starter is the argument (a git URL or a GitHub `owner/repo`), else your `upstream` remote, else `https://github.com/axelhamil/nixos-starter.git`. A fork already holding the tag gets `Already up to date with <tag>.`

It never merges into `main` on its own: the starter's code reaches your machines only once you have read the diff. Adapt your hosts on the branch if needed, wait for `ci-ok`, then merge with a merge commit, never a squash, so the next merge starts from there. On a conflict it names the files, aborts the merge, deletes the branch, returns to where you started and exits 1: create the branch from `origin/main` and merge the tag there by hand, as its error says (`git switch -c chore/sync-upstream-<tag> origin/main && git merge --no-ff <tag>`), resolve and commit, push the branch and open its PR against `main`. A fork that shares no history with the starter (made by copying it, or with "Use this template") gets git's `refusing to merge unrelated histories` and the same clean exit.

To get that PR every Monday at 07:00 UTC instead, set the repository variable `SYNC_UPSTREAM` to `true` (Settings, Secrets and variables, Actions, Variables): the `sync-upstream` workflow then runs `nix run .#upstream -- --yes` on `main` in your fork, and `UPSTREAM` (`owner/repo`) points it at another starter. Without the variable it does nothing, even by hand.

The starter keeps `modules/hosts` and `modules/users` minimal, so conflicts stay rare as long as your own config lives there. The one that comes back is a release editing the examples your fork removed (`modules/hosts/example*`, `modules/users/alice`): git reports a modify/delete conflict, settled by keeping them deleted (`git rm` them, then commit the merge); read their diff first, it shows what a new field looks like. Only the starter publishes releases (`release` is skipped in forks), so `CHANGELOG.md` and the version files never conflict.

`update-flake-lock` runs one hour earlier and opens a PR updating `flake.lock`, which merges itself once `ci-ok` is green: it only moves inputs (nixpkgs, home-manager, ...) to newer revisions of the same sources. Both workflows need the bot App and the settings of [ci.md](ci.md#turn-ci-on-in-your-fork).

## The edit loop

How a change of yours reaches the machine you sit at, then the others:

1. **Edit** your checkout (the host's `flake`, such as `~/nixos`). Tracked files are read as they are on disk, committed or not; a new file must be `git add`ed first, or the flake does not see it.
2. **Try** it with `nh os test`: it builds and switches the running system, but the boot menu keeps its default, so a reboot comes back to what you had.
3. **Switch** with `nh os switch`: the change is live with no reboot, and the boot menu now starts it by default. Reboot only for a new kernel, initrd, disk encryption or bootloader change. `update` pulls your fork first (`git pull --ff-only`), which stops when an incoming commit touches a file you edited: use it once your edits are committed.
4. **Commit and push**, so your other machines follow with `update` or `auto-upgrade`, and CI proves the change on every host.

Every `nh os switch` or `update` leaves a generation in the boot menu: a change that breaks the machine is undone by picking the previous one at boot. `nh os test` leaves none: a reboot returns to the current default.

## A machine follows your fork

Tell the host where it updates from, in its registry entry:

```nix
configurations.nixos.my-laptop.flake = "/home/alice/nixos";
```

**A checkout on the machine** (desktops, laptops): run `update`. It pulls your fork with `git pull --ff-only`, refusing a checkout that has diverged, then runs `nh os switch`, which builds, shows the package diff and switches. Extra arguments go to nh (`update --ask`). `nh os switch` alone works too, since `flake` is also its default. The checkout survives a reboot on its own: in `/home/<you>/` it is kept in `/persist/home/<you>` when the host has `wipe-home`, owned by you; elsewhere on `/` (such as `/etc/nixos`) it is kept in `/persist`, owned by root. Do not list it again in your own `home.persistence` or `environment.persistence`: impermanence refuses a path declared twice. A checkout that is also your `stow.directory` is kept once, by `stow`. The path must be a directory of its own, written without `.`, `..` or a trailing `/`, and not `/`, `/home`, `/nix`, `/persist` or your home itself; the build stops naming `flake` otherwise.

**A flake reference** (servers, Raspberry Pi): set `flake = "github:alice/nixos"` and run `update`, which builds your fork's `main` with no checkout to pull.

## A machine upgrades on its own

Add the `auto-upgrade` feature to a host to have it follow the version CI validated for it. Once every check of a host passes on `main`, CI moves the `deploy/<host>` branch of your fork to that commit ([ci.md](ci.md#validated-versions-per-machine)), with the token of the bot App: without the App, no `deploy/<host>` branch ever exists and the machine stays `waiting`; every day from 04:00 (randomized over 45 minutes, caught up at the next start when the machine was off), `auto-upgrade.service` fetches that branch and builds it. It works with a flake reference (a plain `github:<owner>/<repo>` or `git+https://...`, without a query such as `?dir=`), or with a checkout, whose `origin` remote it reads, an SSH remote turned into its https URL. It trusts that remote only when root or an admin (`wheel`) owns the checkout, its `.git` folder (not a file or a link, as in a worktree) and `.git/config`, and no one outside root and `wheel` can write to them: otherwise any user could point it at a repository of their own. A checkout with no `.git` yet goes straight to the mirrors. When that source is down, it tries the host's `mirrors` in order:

```nix
configurations.nixos.my-server = {
  flake = "github:alice/nixos";
  mirrors = [ "https://codeberg.org/alice/nixos.git" ];
};
```

The fork keeps such a mirror up to date on its own: set the repository variable `MIRROR_URL` to the mirror's https URL (Settings, Secrets and variables, Actions, Variables) and the secret `MIRROR_TOKEN` to a token of the mirror's forge that can push to it, and the `mirror` workflow pushes `main` and every `deploy/<host>` there after each change ([ci.md](ci.md#mirror-to-a-second-forge)). List that same URL in the `mirrors` of each host.

A mirror or an `origin` remote over `http://` or `git://` is refused, since anyone on the network path could then choose what the machine builds. It fetches over https without credentials: a private fork needs a mirror the machine can read without a login. Without `signers`, every source, `mirrors` included, is trusted as much as the fork itself: anyone who can push `deploy/<host>` to one of them decides what root builds on the machine; with them, a source can only withhold a version ([below](#signed-deploy-commits)). On GitHub, protect `deploy/*` with a ruleset whose only bypass is the bot App ([ci.md](ci.md#turn-ci-on-in-your-fork)), and on a mirror give push access to the `MIRROR_TOKEN` account alone.

It applies the new version with `nixos-rebuild boot`: the machine keeps running what it runs, and starts the new version at the next reboot. Set `autoUpgrade.operation = "switch"` in the host's `module` to start it right away. On a host with `disk`, a new version that does not start is abandoned on its own after 3 tries ([install.md](install.md#going-back-to-the-previous-generation)). On a Raspberry Pi (`pi` profile) it builds nothing itself (`autoUpgrade.allowLocalBuild = false`): it substitutes everything from the binary cache and its remote builders, so a Pi upgrades only to a version CI built and cached (`<host>-toplevel`); one the cache does not hold yet waits for the next run.

It never replaces a system it cannot place in the history of `deploy/<host>`:

| Outcome | When | What to do |
|---|---|---|
| `upgraded` | a newer validated commit was built | nothing, it applies at next boot |
| `up-to-date` | `deploy/<host>` is the running commit, or is installed and applies at next boot | nothing, or reboot |
| `waiting` | CI has not validated a version for this host yet (a source answered without `deploy/<host>`), or the cache lacks that version and local builds are off | nothing, the next run retries; if it lasts, CI has not promoted this host: check that the bot App is set up ([ci.md](ci.md#turn-ci-on-in-your-fork)), since without `BOT_CLIENT_ID` the `promote` job never runs, and that the host's checks pass |
| `rolled-back` | the version installed for next boot did not start: its tries are spent and the machine went back to the previous generation, so auto-upgrade does not install it again while `deploy/<host>` points to it | fix it and push, CI will validate the fix; a newer `deploy/<host>` is installed as usual |
| `refused` | the running system or the one installed for next boot was built from uncommitted changes, or from a commit of `main` that `deploy/<host>` does not contain (not pushed yet, or the pointer was moved back by a force push), or the checkout is not owned by an admin, or, on a host with `signers`, no source serves a `deploy/<host>` commit signed for it ([below](#signed-deploy-commits)) | commit and push, let CI validate it; or `chown` the checkout; or check the `DEPLOY_SIGNING_KEY` secret of the fork |
| `unreachable` | no source answered within 60 seconds | check the network or the mirrors; the machine stays on its version |
| `failed` | the build or the switch failed | `journalctl -u auto-upgrade` |

The last run is in `/var/lib/auto-upgrade/state.json` (outcome, from and to commits, source, message, and the package diff summary from `nvd`), one line per run in `/var/lib/auto-upgrade/history.tsv`; both survive reboots. `nixos-version --configuration-revision` prints the commit the running system was built from, with a `-dirty` suffix when the tree had uncommitted changes. When CI signs its promotions, that commit, like the from and to commits, is the promotion commit `deploy/<host>` points to, not the commit of `main` it validated: `git log -1 <commit>` in your fork names the commit of `main` in its subject (`deploy <host>: checks passed for <short sha>`), and that commit is its last parent.

### Signed deploy commits

Whoever can push `deploy/<host>` to one of the machine's sources decides what it builds as root: a leaked bot token, a mirror's forge, or its `MIRROR_TOKEN`. With `signers` set, a machine applies only a commit CI signed for it:

```nix
configurations.nixos.my-server = {
  flake = "github:alice/nixos";
  signers = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA... deploy" ];
};
```

CI signs each promotion with the SSH key in the secret `DEPLOY_SIGNING_KEY` of the fork's `deploy` environment ([ci.md](ci.md#sign-the-deploy-branches)): rather than moving `deploy/<host>` to the commit of `main`, it creates a commit with the same tree, the previous promotion and that commit of `main` as parents, the subject `deploy <host>: checks passed for <short sha>`, and its signature. `nix run .#init` generates that key pair for a new fork: it writes the public key in the `signers` of the hosts it writes or keeps and saves the private key in a file readable by you alone, outside the repository (under `$XDG_RUNTIME_DIR` when set), before it writes any host; it prints only that file's path, with `gh secret set DEPLOY_SIGNING_KEY --env deploy < <file> && shred -u <file>` and the way through the GitHub web page, never the key itself. For a fork made before, generate one yourself (`ssh-keygen -t ed25519 -N "" -C deploy -f deploy`), store `deploy` in the secret, delete it, and put the content of `deploy.pub` in the `signers` of your hosts.

After each fetch, `auto-upgrade` checks the commit with `git verify-commit`, trusting only the keys of `signers` and no GPG or X.509 keyring, then that its subject names this host. A source that fails is skipped with its reason, and the next one is tried: `unsigned`, `not signed with an SSH key`, `signed by a key not in update.signers`, `signature invalid` (changed after signing) or `signed for another host`. When no source serves a commit that passes, the run ends `refused`, naming each source and its reason, builds nothing and leaves the machine on its version; there is no fallback to an unsigned commit. The history check above compares the commits of `main` that promotions carry (their last parent; a commit that is not a promotion carries itself): the one served must be the one the machine runs or a later one. An older signed promotion replayed by a source is therefore refused, while a deploy branch deleted or reset by hand and then promoted again from a later commit of `main` is applied; a source that keeps serving an old signed commit leaves the machine `up-to-date` on it, and `doctor` warns once the last success is more than 14 days old.

Without `signers`, the machine applies whatever its sources serve, signed or not, as before, and `doctor` shows a `Signed deploys` warning at every run. To change the key: add the new public key to `signers` and let the machines upgrade once, still signed by the old key; then replace the secret and remove the old key from `signers`. A machine learns its keys only from its own configuration: a leaked key is revoked by a rebuild on each machine with the key removed, not from CI.

### Know what it did

`sudo doctor` reads that state ([doctor.md](doctor.md)): its `Upgrades` line says when the last run upgraded and what it changed, or since when the machine is up to date, and warns with the reason when the last run was `refused`, `unreachable`, `waiting`, `failed` or `rolled-back`, or when the last success is more than 14 days old. Its `Boot` line warns when the newest generation spent its tries and the machine runs an older one. `doctor --history` prints the last 10 runs.

On a host with the `desktop` or `laptop` profile, the `upgrade-notify` user service runs when a graphical session starts: when the last run says `upgraded` and that upgrade was not announced yet, it shows a `System updated` notification with the package diff summary, then remembers the upgrade (its time and commit) in `~/.local/state/upgrade-notify/seen` (persisted with `wipe-home`), so each upgrade is announced once, reboots included. When the machine does not run the upgraded commit yet (`operation = "boot"`), the notification adds `Restart to use it.` It reads the local state only and makes no network call. On another profile, add `upgrade-notify` to the host's `apps`.
