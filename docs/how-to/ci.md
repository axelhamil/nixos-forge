# CI and repository settings

The few settings a fork needs so its CI runs, how to read the report it writes, and how machines get the versions it validated. Start with [Turn CI on in your fork](#turn-ci-on-in-your-fork). The workflows, every check and the scripts you can run locally are in [reference/ci.md](../reference/ci.md).

## Turn CI on in your fork

Plan about 15 minutes. Steps 1 and 2 are enough for CI to build and test every pull request; the others add the weekly lock PR, safe auto-merge, deploy branches, signing and the binary cache.

1. **Keep `develop`**: untick "Copy the main branch only" when forking, or create `develop` from `main`. Work lands on `develop`, then reaches `main` through a PR.
2. **Enable the workflows**: GitHub disables them on forks. Open the Actions tab and enable them.
3. **Bot GitHub App**: the weekly lock PR, `sync-develop`, `promote` and the optional upstream PRs need an App token, since PRs opened with `GITHUB_TOKEN` trigger no workflow. One App serves all your repositories.
   - [Create a GitHub App](https://github.com/settings/apps/new) with webhooks off.
   - Give it the repository permissions **Contents** and **Pull requests**, read and write, and preferably **Workflows** too ([why](#validated-versions-per-machine)).
   - Generate a private key and install the App on your fork.
   - Set the repository variable `BOT_CLIENT_ID` to its client ID and the secret `BOT_PRIVATE_KEY` to the key.
4. **Allow auto-merge and merge commits**: Settings, General, "Allow auto-merge" (the lock PR) and "Allow merge commits" (the upstream PRs).
5. **Ruleset on `main`**: require a pull request and the `ci-ok` status check. Without a required check, auto-merge merges at once.
6. **Ruleset on `develop`**: block deletions and force pushes, or "Automatically delete head branches" deletes `develop` when a `develop` into `main` PR merges.
7. **Ruleset on `deploy/*`, with the App as the only bypass**. Whoever can push `deploy/<host>` chooses what every machine with `auto-upgrade` builds as root, so only the `promote` job may move these branches. In Settings, Rules, Rulesets, New ruleset, New branch ruleset:
   - name it `deploy` and set Enforcement status to Active;
   - under Bypass list, add your bot App and nothing else, so neither admins nor collaborators can push;
   - under Target branches, add the pattern `deploy/*`;
   - tick **Restrict creations**, **Restrict updates**, **Restrict deletions** and **Block force pushes**.

   Without the App on the bypass list, `promote` prints `refused` with GitHub's reason.
8. **Environment `deploy`, with the signing key (recommended)**. In Settings, Environments, New environment: name it `deploy`, and under Deployment branches and tags choose Selected branches and add `main`. Add the environment secret `DEPLOY_SIGNING_KEY` with the private key file `nix run .#init` names, then `shred -u` that file ([why](#sign-the-deploy-branches)). Only the `promote` job runs in that environment, so no other job, and no branch but `main`, can read the key.
9. **Binary cache (recommended)**: create a public [Cachix](https://www.cachix.org) cache, set the repository variable `CACHIX_CACHE` to its name and the secret `CACHIX_AUTH_TOKEN` to a read and write token.
   - Every job pulls from it with the token, so a private cache works too. Pushes and PRs from a branch of this repository write what they built, except sources and images (SD card, ISO, disk, VM), so the push after a merge rebuilds nothing.
   - A PR from another fork and an upstream sync PR (`chore/sync-upstream-*`) read the cache but never get the token, so code nobody here wrote cannot poison it before review.
   - Without a cache, every run builds every check, and PRs never build kernels.

Scheduled workflows and Dependabot only run from the default branch, so they start once the workflows are on `main`.

To check it worked, open a PR against `develop`: the `check` workflow starts, and the run ends with a report and a green `ci-ok`.

## The CI report

The `report` job runs at the end of every `check` run, even when a job failed or hit its time limit. A run cancelled, for instance by a newer push, writes no report. Where it lands:

- **The run summary** (the run's page in the Actions tab), always.
- **One PR comment**, on a PR from a branch of the same repository, rewritten by each new run. A PR from another fork gets the run summary only, since its token cannot comment.

The comment is posted by the bot App, so it shows under the App's name ([rename it](#rename-the-bot)). The job mints a token from `BOT_CLIENT_ID` and `BOT_PRIVATE_KEY` that may only write pull requests. When the App is not set up or its token cannot be minted (a Dependabot PR gets no Actions secret), the run posts as `github-actions` with its own token.

`gh pr comment --edit-last` rewrites the last comment of the identity that posts, so the App and `github-actions` each keep their own comment:

- A PR open when the App was set up keeps its old `github-actions` comment as it was, and the App posts a new one below, which every later run rewrites. Delete the old one by hand if it gets in the way.
- A run that falls back to `github-actions` updates that identity's comment instead, and the next run with the App updates the App's again.

The job runs `ci-report` from the base branch of the PR (the pushed commit on a push), never from the PR itself. It reads the machines and the check descriptions from the outputs of the `plan` job. A PR whose flake does not evaluate still gets its report: `The flake did not evaluate, so nothing was built or tested.`, with a pointer to the `plan` job log that names the file and line.

Because the report comes from the base branch, a new section of the report appears from the first PR opened after the change that adds it is merged.

A comment over 64 KiB is cut and points to the run summary, which holds up to 1 MiB.

### What each part says

- **The verdict**, in bold, above everything else: `Everything works: every machine builds, boots and passed its tests.`, or how many checks failed or did not finish and which machines would not get the change. A failure in a check that belongs to no machine still keeps CI red.
- **What to do**, under the verdict: `Nothing to fix: this commit is ready for the machines.` on a green run, `Fix what is listed below and push again: CI runs on every push.` on a red one.
- `N tests passed only on their second try and are marked unstable.`: a VM test failed once, then passed on its retry. The run stays green, but a test that keeps showing up here is flaky.
- `The binary cache was unreachable: everything was built from scratch.`: Cachix did not answer, so the run was slower than usual.
- `Checks that compile a kernel run after the merge.`: a PR never compiles a kernel; the push to `develop` after the merge builds those checks.
- `N checks were skipped: they already passed with the same code.`: their result is already in the binary cache.
- **One table per machine**, with each check, what it proves and its result: `passed`, `failed`, `unstable`, `did not finish` (the job was cancelled or hit its time limit before its first attempt ended), or `skipped` with the reason. A retry cut by the time limit keeps the `failed` result and the excerpt of the first attempt. A check that runs on two systems gets one row per system.
- **Module checks**, folded, with the same columns; the block opens by itself when one of them failed.
- **Boot time**: for each machine with a `<host>-boots` check, the `systemd-analyze` total and userspace time, beside the same values on the base revision (the PR's base, or the previous commit on a push). `ci-boot-time` reads the base's values from the binary cache. A value is `n/a` when the check was skipped as already passed, when the base's test is not in the cache, or when the boot did not finish within 120 s. There is no threshold: a shared CI VM is noisy, so read the trend over several runs.
- **Performance against the base**, on a PR ([below](#performance-against-the-base)).
- **Screenshots**: every check whose VM test took a screenshot (`machine.screenshot`) lists its images and the artifact that holds them, `screenshot-<system>-<check>`, kept 7 days. They are evidence to look at, never compared.
- **What went wrong**: for each failed check, the likely cause and what to do, read from `modules/flake/ci/causes.tsv` (the first row whose pattern matches a line of the excerpt wins), the excerpt of the log (the last 20 distinct lines that name an error, each with the two lines before it), and a link to the full log.
- **Package changes per machine**, folded, on a PR: the nvd diff of each host against the base branch.

### Performance against the base

On a PR, the `perf` job boots the base revision's generation and this one in turn, in one VM job, so both sides run on the same runner. It is informational: the `perf` job continues on error, and neither it nor a regression ever turns the run red or blocks the merge.

| Metric | How it is measured | Compared on |
|---|---|---|
| Boot, total and userspace | `systemd-analyze` once the system is up | the best of 5 boots per side |
| Idle RAM | `MemTotal` minus `MemAvailable`, 10 s after boot | the best of 5 boots per side |
| Running services | `systemctl list-units --type=service --state=running` | the median of 5 boots |
| Closure | the size of the host's real system | one value |
| CPU | `sysbench cpu`, 2 threads for 8 s, in events/s | the median of 5 boots |
| Memory bandwidth | `sysbench memory`, 2 threads for 8 s, 1 MiB blocks, in MiB/s | the median of 5 boots |
| Disk, 4k random read | `fio`, 8 s on a 256 MiB file of the root disk, `--direct=1` when the file system allows it, in IOPS | the median of 5 boots |

- The benchmarks run after the idle RAM and services are read, about 30 s per boot. `sysbench` and `fio` come from this version's `pkgs` on both sides, through the store the VM shares with the runner, so only the system under test differs.
- The job runs whenever an `x86_64-linux` host boots in a VM here, so a fork whose hosts upstream lacks gets it too. Every such host that also boots in a VM on the base is measured, each boot a fresh VM of 2 GiB and 2 cores.
- A host absent from the base, or that does not boot in a VM there, is listed under the section as not measured, with the reason; when no host is left, the check writes that list without booting anything.
- A line under the heading names each metric past both its thresholds: boot, RAM, closure and services when they grow, benchmarks (`better: "higher"`) when they drop. The source of these values is `perfThresholds` in `modules/flake/ci/ci-report.jq`; the benchmark thresholds are twice the worst spread measured between two identical sides on a local machine, and a shared CI runner is noisier, so read their trend over several runs:

  | Metric | Flagged when it moves the wrong way by more than | and by at least |
  |---|---|---|
  | Boot, total | 10 % | 0.5 s |
  | Boot, userspace | 10 % | 0.5 s |
  | Idle RAM | 10 % | 32 MiB |
  | Closure | 5 % | 50 MiB |
  | Running services | 0 % | 1 service |
  | CPU | 5 % | 100 events/s |
  | Memory bandwidth | 5 % | 1000 MiB/s |
  | Disk, 4k random read | 10 % | 2000 IOPS |

- A value is `n/a` when one side was not measured. The section is left out on a push, or when the job failed or produced nothing readable.
- The raw samples are in the `perf-<system>` artifact. The base is the `migration-base` input, overridden by `ci-build --base` with the PR's base commit.

## Rename the bot

The comments and PRs come from your bot GitHub App ([setup](#turn-ci-on-in-your-fork)). To give it a name and a face, open the App's settings (Settings, Developer settings, GitHub Apps, Edit):

- change its **GitHub App name** (it must be unique on GitHub);
- under **Display information**, upload a logo, which becomes its avatar.

Its client ID does not change, so `BOT_CLIENT_ID` stays as it is. The CI report comment, the lock PR and the upstream PR then all show under that name.

## Validated versions per machine

Each machine has a branch `deploy/<host>` that points to the last commit of `main` on which every check of that machine passed. A machine follows its own branch rather than `main`, so a commit that breaks one machine never reaches it, while the others still move on.

The `promote` job runs after every push to `main`, once the builds ended, even when some failed or hit their time limit; a cancelled run promotes nothing. It reads the exact list of checks of each machine from `nix eval --json .#ci.hostChecks`, never a name prefix, so `<host>-server-boots` failing never holds `deploy/<host>` back.

- A check counts as passed when its result is `passed` or `unstable`, or when the plan skipped it because the binary cache already holds it.
- A check without a result (its job was cancelled or hit its time limit), or skipped for any other reason, keeps the branch where it is.

`ci-promote` prints one line per machine, after a first line `unsigned: ...` when `DEPLOY_SIGNING_KEY` is not set:

- `deploy/<host> -> <short sha>`: the branch moved to the pushed commit.
- `deploy/<host> -> <short sha>, signed as <short sha>`: the branch moved to a new promotion commit carrying the pushed commit, signed ([below](#sign-the-deploy-branches)).
- `deploy/<host> refused: DEPLOY_SIGNING_KEY cannot sign, it must hold an OpenSSH private key without a passphrase`: the secret holds something else; the branch stays and the job fails.
- `deploy/<host> warning: its tip <short sha> is not a promotion signed by DEPLOY_SIGNING_KEY, ...`: the branch holds a commit the key did not sign for that host (pushed by hand, reset to a commit of `main`, or forged). The new promotion replaces it rather than building on it, with `--force-with-lease` on that tip, and the line shows up as a warning of the run. Find out who moved the branch.
- `deploy/<host> kept: <check> <outcome>`: a check of that machine did not pass, the branch stays.
- `deploy/<host> refused: not a fast-forward`: the pushed commit does not descend from the branch (for a promotion commit, from the commit of `main` it carries), for instance after a force push on `main`. The branch stays and the job fails; `ci-ok` is not affected.
- `deploy/<host> already at or past <short sha>`: the branch already holds the pushed commit, or a later one, for instance when an older run is rerun after a newer one promoted the machine. Nothing moves and the job stays green.
- `deploy/<host> refused: <reason>`: the remote declined the push, with git's reason, for instance the `deploy/*` ruleset when the App is not on its bypass list.

The job is skipped when `BOT_CLIENT_ID` is not set. Its token asks for the **Contents** write permission only, whatever else the App installation grants. GitHub may refuse to move a branch across commits that change `.github/workflows/` to a token without the **Workflows** permission; `ci-promote` then prints the refusal reason and the branch stays.

## Sign the deploy branches

With the secret `DEPLOY_SIGNING_KEY` (an OpenSSH private key without a passphrase, ed25519 from `nix run .#init`), `promote` does not move `deploy/<host>` to the commit of `main` itself. It creates a promotion commit with `git commit-tree -S`:

- the tree of that commit of `main`;
- the subject `deploy <host>: checks passed for <short sha>`;
- two parents, the previous promotion of `deploy/<host>` and the commit of `main` (only the latter for the first one);
- an SSH signature.

The branch therefore only moves forward, each promotion holds the previous one, and the ruleset above still refuses anything else.

Before building on the tip of `deploy/<host>`, `promote` checks it against the public key it derives from the secret (`ssh-keygen -y`). A tip that is not a promotion signed for that host is neither a parent nor a reason to answer `already at or past`, and the run warns about it. So a branch deleted or reset by hand never holds the machines back.

Machines whose host lists the public key in `signers` apply only such commits, signed for them ([updates.md](../explanation/updates.md#signed-deploy-commits)). Without the secret, `promote` prints `unsigned: DEPLOY_SIGNING_KEY is not set, ...` and moves the branches to the commits of `main`, unsigned, which those machines refuse.

How the key is kept:

- It is written to a file only the job's user can read, in a temporary folder removed when `ci-promote` ends, and is never printed.
- It lives in the `deploy` environment, limited to `main` ([setup](#turn-ci-on-in-your-fork)), apart from the App token. A leaked token can push a deploy branch but not sign it, and a leaked key can sign but not push. Anyone who can run a workflow on `main` of the fork can use both: that is the accepted limit.
- Signing happens once, in `promote`, before any push. The `mirror` workflow copies the signed commits as they are and never creates or rewrites a commit, so a mirror's forge can withhold a version but not forge one.

## Mirror to a second forge

The `mirror` workflow copies `main` and the `deploy/*` branches to another forge (Codeberg, a Forgejo, Gitea or GitLab instance, or another GitHub repository), which machines list in their `mirrors` ([updates.md](../explanation/updates.md#a-machine-upgrades-on-its-own)). It runs on every push to `main` and to `deploy/*`. The `promote` job pushes `deploy/*` with the App token, and pushes made with an App token trigger workflows, unlike those made with `GITHUB_TOKEN`, so each promotion reaches the mirror. Runs queue one after the other, and each one pushes the branches as they are when it starts.

The mirror holds the fork's commits, not its inputs. While GitHub is down, a `deploy/<host>` whose `flake.lock` is the one the machine already runs builds from the mirror. One whose `flake.lock` changed, as after every weekly lock update, needs its new `github:` inputs, which neither the mirror nor the binary cache hold (Cachix keeps no sources). That build fails, the run is recorded as `failed`, and the next daily run retries.

To set it up:

1. Set the repository variable `MIRROR_URL` to the https URL of the mirror repository.
2. Set the secret `MIRROR_TOKEN` to an access token of the mirror's forge that can push to it.

`ci-mirror` sends the token with HTTP Basic auth, as the password of the user `x-access-token`, which GitHub, GitLab, Gitea, Forgejo and Codeberg accept. The header is read from the environment, never on the command line or in the log, and sent to the mirror URL only. Without `MIRROR_URL` the workflow is skipped; without `MIRROR_TOKEN` it pushes without credentials, which suits a remote that needs none.

`ci-mirror <url>` pushes only `main` and `deploy/*`, never `develop` or any other branch:

- `main` moves without force: a mirror whose `main` has diverged refuses the push, and the job fails and needs a look, while the `deploy/*` branches still go through.
- `deploy/*` is pushed with force, and a `deploy/<host>` deleted in the fork is deleted on the mirror; nothing else on the mirror is pruned. The mirror holds a copy of the fork's pointer, so a machine reading the mirror never follows a version the fork withdrew.

## Live badges

The README can show badges that follow every run instead of going stale. They read [shields.io endpoint](https://shields.io/badges/endpoint-badge) files from the orphan branch `badges`:

| File | Written by | Shows |
|---|---|---|
| `checks.json` | the `badges` job of `check`, on a push to `main` | `36/36 passing`, or the failing count in red |
| `vm-tests.json` | the same | boot and VM tests |
| `hosts.json` | the same | hosts built |
| `boot-time.json` | the same | the time to `multi-user.target` of the host named by the `BADGE_HOST` environment variable of `ci-badges` (`example` by default), else of the first host, by name, with a boot test <!-- docs-check: allow example --> |
| `reproducibility.json` | the nightly `reproducibility` workflow | `reproducible` or `N paths differ` |

`nix run .#ci-badges -- publish badges` commits them to `badges` with `GITHUB_TOKEN`, the only place where a job gets `contents: write`. Publishing never turns a check red: a failure is a warning on the step.

A fork gets its own badges once its CI has run on `main`. `nix run .#init` leaves the README's badges pointing at upstream's repository: point each badge URL at `https://raw.githubusercontent.com/<owner>/<repo>/badges/<file>.json`, and the workflow badges at your own repository.
