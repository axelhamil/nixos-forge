# CI and repository settings

What the GitHub Actions workflows do, what they prove, and the few settings a fork needs so they run. Start with [Turn CI on in your fork](#turn-ci-on-in-your-fork); the rest is here for when you want to know why the report says what it says.

## Turn CI on in your fork

1. **Keep `develop`**: untick "Copy the main branch only" when forking, or create `develop` from `main`. Work lands on `develop`, then reaches `main` through a PR.
2. **Enable the workflows**: GitHub disables them on forks. Open the Actions tab and enable them.
3. **Bot GitHub App**: the release PR, the weekly lock PR and the optional upstream PRs need an App token, since PRs opened with `GITHUB_TOKEN` trigger no workflow. [Create a GitHub App](https://github.com/settings/apps/new) with webhooks off and the repository permissions **Contents** and **Pull requests** set to read and write, and preferably **Workflows** too (see [below](#validated-versions-per-machine)), generate a private key, install the App on your fork, then set the repository variable `BOT_CLIENT_ID` to its client ID and the secret `BOT_PRIVATE_KEY` to the key. One App serves all your repositories.
4. **Allow auto-merge and merge commits**: Settings, General, "Allow auto-merge" (the lock PR) and "Allow merge commits" (the upstream PRs).
5. **Ruleset on `main`**: require a pull request and the `ci-ok` status check. Without a required check, auto-merge merges at once.
6. **Ruleset on `develop`**: block deletions and force pushes, or "Automatically delete head branches" deletes `develop` when a `develop` into `main` PR merges.
7. **Ruleset on `deploy/*`, with the App as the only bypass**: whoever can push `deploy/<host>` chooses what every machine with `auto-upgrade` builds as root, so only the `promote` job may move these branches. Settings, Rules, Rulesets, New ruleset, New branch ruleset: name it `deploy`, set Enforcement status to Active, under Bypass list add your bot App (and nothing else, so neither admins nor collaborators can push), under Target branches add the pattern `deploy/*`, and tick **Restrict creations**, **Restrict updates**, **Restrict deletions** and **Block force pushes**. Without the App on the bypass list, `promote` prints `refused` with GitHub's reason.
8. **Environment `deploy`, with the signing key (recommended)**: Settings, Environments, New environment: name it `deploy`, under Deployment branches and tags choose Selected branches and add `main`, then add the environment secret `DEPLOY_SIGNING_KEY` with the private key file `nix run .#init` names, then `shred -u` that file ([below](#sign-the-deploy-branches)). Only the `promote` job runs in that environment, so no other job, and no branch but `main`, can read the key.
9. **Binary cache (recommended)**: create a public [Cachix](https://www.cachix.org) cache, set the repository variable `CACHIX_CACHE` to its name and the secret `CACHIX_AUTH_TOKEN` to a read and write token. Every job pulls from it, and pushes and PRs from a branch of this repository write what they built, except sources and images (SD card, ISO, disk, VM), so the push after a merge finds the PR's builds and rebuilds nothing. A PR from another fork and an upstream sync PR (`chore/sync-upstream-*`) read the cache but never get the token, so code nobody here wrote cannot poison it before review. Without it, every run builds every check and PRs never build kernels.

Scheduled workflows and Dependabot only run from the default branch, so they start once the workflows are on `main`.

## Workflows

| Workflow | Trigger | What it does |
|---|---|---|
| `check` | every PR, push to `main` and `develop`; a newer push cancels the running check of its PR or of `develop`, never of `main` | Lints the flake, plans the build with `ci-plan` (checks already in the binary cache are skipped), builds each remaining check on a runner of its system (`ubuntu-24.04` with KVM, or `ubuntu-24.04-arm`), retrying a VM test once, computes the package diff ([nvd](https://git.sr.ht/~khumba/nvd)) per system on a PR, and writes one readable report ([below](#the-ci-report)). `ci-ok` sums up the result. On a push to `main`, `promote` moves `deploy/<host>` to the pushed commit for every machine whose checks all passed ([below](#validated-versions-per-machine)), and `badges` publishes the live badges ([below](#live-badges)). |
| `mirror` | push to `main` or `deploy/*`, when `MIRROR_URL` is set | Pushes `main` and every `deploy/<host>` to a second forge, so machines can still fetch their validated version when GitHub is down; a version whose `flake.lock` changed still needs its inputs from GitHub ([below](#mirror-to-a-second-forge)). |
| `reproducibility` | every day 03:00 UTC, or by hand | Runs `nix run .#reproducibility`: builds the system of every `x86_64-linux` machine, builds it again with `nix build --rebuild` and writes to the run summary whether the two match, with a [diffoscope](https://diffoscope.org) summary when they do not ([below](#reproducibility)). On `main`, it publishes the `reproducibility` badge. Informational: a difference fails nothing, and `ci-ok` never waits for it. |
| `update-flake-lock` | Monday 06:00 UTC, or by hand | Opens a PR updating `flake.lock` against `main`, with auto-merge on; [Narvi](#narvi) opens its body with a line and a summary of the inputs that moved. |
| `sync-develop` | push to `main` | Merges `main` into `develop` with the App token, so `check` runs on the merged `develop`. |
| `sync-upstream` | Monday 07:00 UTC, or by hand, forks only, when the repository variable `SYNC_UPSTREAM` is `true` | Runs `nix run .#upstream -- --yes` on `main`: opens a PR merging the starter's latest release, for you to review and merge ([updates.md](updates.md#your-fork-follows-the-starter)). The variable `UPSTREAM` (`owner/repo`) points it at another starter. |
| `release` | push to `main`, starter only | Runs [release-please](https://github.com/googleapis/release-please): a release PR bumps the version and `CHANGELOG.md` from Conventional Commits; merging it tags and publishes the GitHub Release, whose notes [Narvi](#narvi) then opens with a line and a summary of what the release means for a machine. |
| Dependabot | weekly | Updates GitHub Actions versions, with PRs against `develop`. |

A check that compiles a kernel is only evaluated in a PR and built on the push to `develop` or `main` after the merge.

## The CI report

The `report` job runs at the end of every `check` run, even when a job failed or hit its time limit; a run cancelled, for instance by a newer push, writes no report. Where it lands:

- **The run summary** (the run's page in the Actions tab), always.
- **One PR comment**, on a PR from a branch of the same repository, rewritten by each new run. A PR from another fork gets the run summary only, since its token cannot comment.

The comment is posted by the bot App, so it shows as [Narvi](#narvi), with a token the job mints from `BOT_CLIENT_ID` and `BOT_PRIVATE_KEY` that may only write pull requests. When the App is not set up or its token cannot be minted (a Dependabot PR gets no Actions secret), the run posts as `github-actions` with its own token.

`gh pr comment --edit-last` rewrites the last comment of the identity that posts, so the App and `github-actions` each keep their own comment:

- A PR open when the App was set up keeps its old `github-actions` comment as it was at its last run, and the App posts a new one below, which every later run rewrites. That old comment is never updated again: delete it by hand if it gets in the way.
- A run that falls back to `github-actions` updates that identity's comment instead, and the next run with the App updates the App's again.

The job runs `ci-report` from the base branch of the PR (the pushed commit on a push), never from the PR itself, and reads the machines and the check descriptions from the outputs of the `plan` job. A PR whose flake does not evaluate still gets its report: `The flake did not evaluate, so nothing was built or tested.`, with a pointer to the `plan` job log that names the file and line.

Line by line:

- **Narvi's line**, in italics, above everything else: one line of `messages.json` for a green or a red run ([below](#narvi)). It is left out when the list is empty.
- **The verdict**, in bold: `Everything works: every machine builds, boots and passed its tests.`, or how many checks failed or did not finish and which machines would not get the change. A failure in a check that belongs to no machine still keeps CI red.
- `N tests passed only on their second try and are marked unstable.`: a VM test failed once, then passed on its retry. The run stays green, but a test that keeps showing up here is flaky.
- `The binary cache was unreachable: everything was built from scratch.`: Cachix did not answer, so the run was slower than usual.
- `Checks that compile a kernel run after the merge.`: a PR never compiles a kernel; the push to `develop` after the merge builds those checks.
- `N checks were skipped: they already passed with the same code.`: their result is already in the binary cache.
- **One table per machine**, with each check, what it proves and its result: `passed`, `failed`, `unstable`, `did not finish` (the job was cancelled or hit its time limit before its first attempt ended), or `skipped` with the reason. A retry cut by the time limit keeps the `failed` result and the excerpt of the first attempt. A check that runs on two systems gets one row per system, with the system after its name.
- **Module checks**, folded, with the same columns; the block opens by itself when one of them failed.
- **Boot time**: for each machine with a `<host>-boots` check, the `systemd-analyze` total and userspace time its boot test recorded, beside the same values on the base revision (the PR's base, or the previous commit on a push), read from the base's `<host>-boots` in the binary cache by `ci-boot-time`. A value is `n/a` when the check did not run in this run (skipped as already passed), when the base's test is not in the cache (no Cachix, or a base older than the boot time), or when the boot did not finish within 120 s. There is no threshold: a shared CI VM is noisy, so read the trend over several runs. For reference, `example` measured 7.0 s in total (0.4 s kernel, 3.6 s initrd, 3.0 s userspace) on a local machine on 17/10/2026. The section is left out when no machine has a value.
- **Screenshots**: every check whose VM test took a screenshot (`machine.screenshot`, such as `desktop` in `app-hyprland`) lists its images and the artifact that holds them, `screenshot-<system>-<check>`, kept 7 days on the run's page. They are evidence to look at, never compared. A check skipped as already passed takes no new screenshot.
- **What went wrong**: for each failed check, the likely cause and what to do, read from `modules/flake/ci/causes.tsv` (the first row, top to bottom, whose pattern matches a line of the excerpt wins), the excerpt of the log (the last 20 distinct lines that name an error, each with the two lines before it), and a link to the full log.
- **What changes on the machines**, on a PR whose package diff is in the report and when [Narvi](#narvi) can call Claude Haiku: 2 to 4 sentences on what the diff changes that a person cares about (the kernel, the desktop, notable version jumps, removals), written from the diff alone.
- **Package changes per machine**, folded, on a PR: the nvd diff of each host against the base branch.

The report job runs `ci-report` and `ci-boot-time` from the base branch, so the boot time, screenshot and Narvi sections appear from the first PR opened after the change that adds them is merged; the PR carrying that change gets its report without them.

A comment over 64 KiB is cut and points to the run summary, which holds up to 1 MiB.

## Narvi

Narvi, the dwarf who made the Doors of Durin, is the voice of the CI. He speaks where the CI already writes, and only there: facts first, at most one line of lore at the top, never inside a failure, a table or the breaking changes. Speak, friend, and read the diff.

| Where | What Narvi says |
|---|---|
| The CI report (run summary and PR comment) | a line for a green or a red run above the verdict, and on a PR a summary of the package diff above the folded nvd output |
| The weekly lock PR | a line, then a summary of the inputs that moved, above the body `update-flake-lock` writes |
| The upstream sync PR of a fork (`nix run .#upstream`) | a line, then a summary of the starter's release, above the breaking changes and the changelog, which stay as they are |
| The GitHub Release | a line, then what the release means for your machine, above the notes release-please generated |

Everything runs through `nix run .#narvi` (`modules/flake/ci/narvi/`): `say <moment> [seed]` prints a line, `summarize <kind> <file>` a summary, `preface <moment> <kind> <file> [seed]` both as Markdown, `pr <moment> <kind> <number>` and `release <tag>` put that preface on top of a PR body or of a release's notes, once: a rerun finds the `<!-- narvi -->` marker and changes nothing.

### Change what Narvi says

Every line lives in `modules/flake/ci/narvi/messages.json`, one list per moment: `report-green`, `report-red`, `lock-pr`, `upstream-pr`, `release`. Edit, add or remove lines; the `narvi` check wants 5 to 10 plain ASCII lines per moment, without dashes or Markdown. The line is picked from the run (`GITHUB_RUN_ID`, else `GITHUB_SHA`; the PR number or the tag for PRs and releases), so a rerun says the same. An empty list makes Narvi silent at that moment, never an error.

### Turn the summaries on

Summaries come from Claude Haiku (`claude-haiku-5-5`) through the Messages API, with a system prompt that keeps it to the facts of its input, in plain prose. They cost a few short Haiku calls per run: one per report (the run summary and the PR comment share it), one per lock PR, upstream PR and release. Without credentials Narvi says his line and nothing else. Two ways, the first that works wins:

1. **Workload Identity Federation (recommended, no stored key)**: in the [Claude Console](https://platform.claude.com/settings/workload-identity-federation), connect a GitHub Actions workload: the issuer `https://token.actions.githubusercontent.com` and a service account. Then set the repository variables `ANTHROPIC_FEDERATION_RULE_ID`, `ANTHROPIC_ORGANIZATION_ID`, `ANTHROPIC_SERVICE_ACCOUNT_ID` and, when the rule covers more than one workspace, `ANTHROPIC_WORKSPACE_ID`. The jobs that call Narvi ask GitHub for an OIDC token with the audience `https://api.anthropic.com`, trade it at `POST /v1/oauth/token` for a short-lived access token, and send that as a bearer token. Make two federation rules on that service account, each with the claims `repository_id` (from `gh api repos/<owner>/<repo> --jq .id`) and `repository_owner` set to yours:
   - **pushes**: `subject_prefix` `repo:<owner>/<repo>:ref:refs/heads/`, which matches runs on a branch of your repository only, so only people who can push to it: the lock PR, the upstream PR, the release and the reports of pushes to `main` and `develop`. Add the claim `ref` set to `refs/heads/main` to keep `develop` and other branches out, at the cost of no summary in the reports of `develop`. Its id goes in `ANTHROPIC_FEDERATION_RULE_ID`.
   - **pull requests**, optional: `subject_prefix` `repo:<owner>/<repo>:pull_request`, for the reports of PRs. Its id goes in `ANTHROPIC_PR_FEDERATION_RULE_ID`, which the `report` job uses on PRs instead of the first; without it, PR reports have no summary through federation. Anthropic warns that such a rule matches PRs from forks too: GitHub gives a fork PR no OIDC token only while "Send write tokens to workflows from fork pull requests" is off (Settings, Actions, General, offered on private repositories; public ones never send them). With it on, a fork PR could edit `check.yml` in its own branch and trade the token for Anthropic access. Create this rule only while that setting is off.
2. **An API key**: the repository secret `ANTHROPIC_API_KEY`.

Narvi never turns CI red and never blocks:

- **Any failure means no summary, not a red run.** No credentials, an HTTP error, no answer within 30 s for all its calls together (GitHub's token, the exchange and the message), an answer that is not a message or holds no text: each gives no summary, a warning on the job's log and exit code 0.
- **A failure is remembered** for the rest of the job, so the PR comment does not wait again after the run summary waited.
- **A summary is flattened** to one plain paragraph before it is posted: HTML and comments are dropped, links and images keep their text only, Markdown characters are escaped, and `@` mentions and URLs are broken with a zero-width space. A diff or a commit subject that steers Haiku can neither hide what follows, ping anyone nor embed a link under Narvi's name.
- **Secrets stay hidden.** Tokens and keys travel in header files only the job's user can read, never on a command line or in the log.
- **A PR from another fork** gets no OIDC token and no secret, so its report has the line alone.

The diff Haiku reads comes from the PR: a summary is a hint to read the diff below it, never a verdict.

### Rename the bot

The comments and PRs come from your bot GitHub App ([setup](#turn-ci-on-in-your-fork)). To make it Narvi, open the App's settings (Settings, Developer settings, GitHub Apps, Edit): change its **GitHub App name** (it must be unique on GitHub, so `narvi-<your login>` if `Narvi` is taken), and under **Display information** upload a logo, which becomes its avatar. Its client ID does not change, so `BOT_CLIENT_ID` stays as it is. The CI report comment, the lock PR, the upstream PR and the release are then all Narvi's; the release notes are edited with the run's token, which leaves the release's author unchanged.

## Validated versions per machine

Each machine has a branch `deploy/<host>` that points to the last commit of `main` on which every check of that machine passed. A machine follows its own branch rather than `main`, so a commit that breaks one machine never reaches it, while the others still move on.

The `promote` job runs after every push to `main`, once the builds ended, even when some failed or hit their time limit; a cancelled run promotes nothing. It reads the exact list of checks of each machine from `nix eval --json .#ci.hostChecks`, never a name prefix, so `example-server-boots` failing never holds `deploy/example` back. A check counts as passed when its result is `passed` or `unstable`, or when the plan skipped it because the binary cache already holds it. A check without a result (its job was cancelled or hit its time limit) or skipped for any other reason keeps the branch where it is.

`ci-promote` prints one line per machine, after a first line `unsigned: ...` when `DEPLOY_SIGNING_KEY` is not set:

- `deploy/<host> -> <short sha>`: the branch moved to the pushed commit.
- `deploy/<host> -> <short sha>, signed as <short sha>`: the branch moved to a new promotion commit carrying the pushed commit, signed ([below](#sign-the-deploy-branches)).
- `deploy/<host> refused: DEPLOY_SIGNING_KEY cannot sign, it must hold an OpenSSH private key without a passphrase`: the secret holds something else; the branch stays and the job fails.
- `deploy/<host> warning: its tip <short sha> is not a promotion signed by DEPLOY_SIGNING_KEY, ...`: the branch holds a commit the key did not sign for that host (pushed by hand, reset to a commit of `main`, or forged); the new promotion replaces it rather than building on it, with `--force-with-lease` on that tip, and the line shows up as a warning of the run. Find out who moved the branch.
- `deploy/<host> kept: <check> <outcome>`: a check of that machine did not pass, the branch stays.
- `deploy/<host> refused: not a fast-forward`: the pushed commit does not descend from the branch (from the commit of `main` it carries, for a promotion commit), for instance after a force push on `main`. The branch stays, the job fails and needs a look; `ci-ok` is not affected.
- `deploy/<host> already at or past <short sha>`: the branch already holds the pushed commit (with the signing key, a promotion that verifies with it carries that commit or a later one), for instance when an older run is rerun after a newer one promoted the machine. Nothing moves and the job stays green.
- `deploy/<host> refused: <reason>`: the remote declined the push, with git's reason, for instance the `deploy/*` ruleset when the App is not on its bypass list.

The job is skipped when `BOT_CLIENT_ID` is not set. Its token asks for the **Contents** write permission only, whatever else the App installation grants. GitHub may refuse to move a branch across commits that change `.github/workflows/` to a token without the **Workflows** permission; `ci-promote` then prints the refusal reason and the branch stays.

## Sign the deploy branches

With the secret `DEPLOY_SIGNING_KEY` (an OpenSSH private key without a passphrase, ed25519 from `nix run .#init`), `promote` does not move `deploy/<host>` to the commit of `main` itself. It creates a promotion commit with `git commit-tree -S`:

- the tree of that commit of `main`;
- the subject `deploy <host>: checks passed for <short sha>`;
- two parents, the previous promotion of `deploy/<host>` and the commit of `main` (only the latter for the first one);
- an SSH signature.

The branch therefore only moves forward, each promotion holds the previous one, and the ruleset above still refuses anything else. Before building on the tip of `deploy/<host>`, `promote` checks it against the public key it derives from the secret (`ssh-keygen -y`): a tip that is not a promotion signed for that host is neither a parent nor a reason to answer `already at or past`, and the run warns about it, so a branch deleted or reset by hand never holds the machines back. Machines whose host lists the public key in `signers` apply only such commits, signed for them ([updates.md](updates.md#signed-deploy-commits)). Without the secret, `promote` prints `unsigned: DEPLOY_SIGNING_KEY is not set, ...` and moves the branches to the commits of `main`, unsigned, which those machines refuse.

The key is written to a file only the job's user can read, in a temporary folder removed when `ci-promote` ends, and is never printed. It lives in the `deploy` environment, limited to `main` ([setup](#turn-ci-on-in-your-fork)), apart from the App token: a leaked token can push a deploy branch but not sign it, and a leaked key can sign but not push. Anyone who can run a workflow on `main` of the fork can use both: that is the accepted limit. Signing happens once, in `promote`, before any push: the `mirror` workflow copies the signed commits as they are and never creates or rewrites a commit, so a mirror's forge can withhold a version but not forge one.

## Mirror to a second forge

The `mirror` workflow copies `main` and the `deploy/*` branches to another forge (Codeberg, a Forgejo, Gitea or GitLab instance, or another GitHub repository), which machines list in their `mirrors` ([updates.md](updates.md#a-machine-upgrades-on-its-own)). It runs on every push to `main` and to `deploy/*`; the `promote` job pushes `deploy/*` with the App token, and pushes made with an App token do trigger workflows, unlike those made with `GITHUB_TOKEN`, so each promotion reaches the mirror. Runs queue one after the other and each one pushes the branches as they are when it starts.

The mirror holds the fork's commits, not its inputs. While GitHub is down, a `deploy/<host>` whose `flake.lock` is the one the machine already runs builds from the mirror; one whose `flake.lock` changed, as after every weekly lock update, needs its new `github:` inputs, which neither the mirror nor the binary cache hold (Cachix keeps no sources). That build fails, the run is recorded as `failed`, and the next daily run retries.

Set the repository variable `MIRROR_URL` to the https URL of the mirror repository, and the secret `MIRROR_TOKEN` to an access token of the mirror's forge that can push to it. `ci-mirror` sends it with HTTP Basic auth, the token as the password and `x-access-token` as the user name (the header `Authorization: Basic <base64 of x-access-token:<token>>`), which GitHub, GitLab, Gitea, Forgejo and Codeberg accept, since they take the user name from the token. The header is read from the environment, never on the command line or in the log, and sent to the mirror URL only. Without `MIRROR_URL` the workflow is skipped; without `MIRROR_TOKEN` it pushes without credentials, which suits a remote that needs none.

`ci-mirror <url>` pushes only `main` and `deploy/*`, never `develop` or any other branch:

- `main` moves without force: a mirror whose `main` has diverged refuses the push, the job fails and needs a look, while the `deploy/*` branches still go through.
- `deploy/*` is pushed with force, and a `deploy/<host>` deleted in the fork is deleted on the mirror; nothing else on the mirror is pruned. A deploy branch is the fork's pointer, which only `promote` moves once the ruleset above is in place, or an admin who lifts it: the mirror holds a copy of it, so a branch moved back or removed by hand in the fork is moved back or removed on the mirror too, and a machine reading the mirror never follows a version the fork withdrew.

## Checks

Every host gets its checks from the registry, so a new host reaches CI without touching YAML.

| Check | When | What it proves |
|---|---|---|
| `<host>-toplevel` | every host | the system builds, with its real revision; its closure is what `auto-upgrade` substitutes |
| `<host>-boots` | every host without an SD image | the VM reaches `multi-user.target` and every `home-manager-<user>` service; once the boot finished, it writes `systemd-analyze`'s total and userspace time to `boot-time.json` in its output, without failing when the boot does not finish |
| `<host>-sd-image` | Raspberry Pi hosts, in place of `<host>-boots` | the SD card image builds |
| `<host>-disk` | hosts with `disk` | install on a blank disk, reboot, `/` wiped, btrbk snapshot taken; Secure Boot and TPM2 unlock on `x86_64-linux` hosts with `secure-boot`; the swap file, with a 1G size, on hosts with `hibernate` |
| `<host>-wipe-home` | hosts with `disk` but not `wipe-home` | the install test with `wipe-home` added |
| `<host>-installer` | hosts in `installer.testedHosts` | the installer ISO installs without questions and the installed disk boots |
| `base-*`, `feature-*`, `profile-*`, `app-*` | modules with a `check.nix` | the module's own VM test |
| `feature-migration` | while `example` exists and imports `disk` here and on the base, on `x86_64-linux`; built on every run, never skipped as cached | the `example` host of the base installs on a disk and writes state in `/persist`, `/var/lib/nixos` and a home; this version's `example` is installed with `switch-to-configuration boot` and started; the machine-id, the user's uid and every file are still there (a home that this version starts wiping is found in the wiped home's snapshot, as [wipe-home.md](wipe-home.md#turn-it-on-for-a-home-in-use) says), `doctor` loses no file and sees no failed unit but the `home-manager-<user>` services, which never start on the store the test shares over 9p, and the `systemd-boot-check-no-failures` they fail; Secure Boot and TPM2 are left to `example-disk`. The base is the `migration-base` input: in a PR, `ci-build --base` overrides it with the PR's base commit, on a push with the commit before the push (a push that creates the branch has none and keeps the locked input), in the repository the run belongs to, a fork included; locally it is the starter's `main` as locked. `init` leaves the input in a fork, since CI overrides it there |
| `feature-migration-without-disk` | always, on `x86_64-linux` | a minimal machine without `disk`, as this version or as the base, gets no `feature-migration` and the flake still evaluates, so an `example` without `disk` keeps `nix flake check` green |
| `feature-rollback-failed-unit` | always, on `x86_64-linux` | like `feature-rollback`, with a generation whose service fails after `multi-user.target`: `auto-upgrade`'s `systemd-boot-check-no-failures` keeps the boot from being blessed |
| `wrong-namespace` | always, on `x86_64-linux` | a host naming in `features`, `profile` or `hardware` a name of another field fails to evaluate, with an error naming that field and listing only its own names; a name in `apps` that is neither an app nor a nixpkgs package lists the configured apps; a `users` entry with no person in `modules/users` lists the known people |
| `apps-once` | always, on `x86_64-linux` | apps named by both the host and the person (`zsh`, `hyprland`, `lock`) build the same system as each one once, with a hyprlock PAM service |
| `hyprland-alone` | always, on `x86_64-linux` | `hyprland` alone evaluates, binds its terminal key to `xdg-terminal-exec`, warns that no terminal is registered, and binds no key to `fuzzel`, the lock or `wpctl`; with `kitty`, `fuzzel`, `lock` and the `audio` hardware module it binds them all, kitty is the default terminal and the warning is gone |
| `flake-modules` | always, on `x86_64-linux` | no name is published by two namespaces in the `flake.modules.nixos` output |
| `facter` | always, on `x86_64-linux` | a host with the registry field `facter` set to nixpkgs' sample report turns on nixpkgs' facter module |
| `options-doc` | always | `docs/options.md` matches the registry options |
| `init` | while the `example` host exists | `nix run .#init -- --yes` on a copy of the flake writes a host and a person that parse, with the answers and features asked, removes the examples, renames the example person in the hosts it keeps, refuses to overwrite a host or a person, writes the same new deploy signing key in the hosts it writes and keeps, saves the private key in a 0600 file of a 0700 folder under `XDG_RUNTIME_DIR` and prints only its path in the `gh secret set` and `shred -u` command, and evaluates the `toplevel` of every host left with the flake's locked inputs; an unknown package in `--apps`, a person field of the wrong type and an unknown option in a person's `module` make it fail |
| `example-hosts` | while an example host exists, on `x86_64-linux` | every example host has a line, enabled or commented out, for every registry field, profile, feature, hardware module and app of the starter, and the person the `example` host names has one for every person field |
| `ci-promote` | always, on `x86_64-linux` | on local git repositories, `deploy/<host>` moves only when every check of that host passed, unstable or cached, stays on a failure, a missing result or a kernel skip, never follows another host sharing its name prefix, and is never moved backwards or sideways, says when it is already at or past the commit, and prints the reason the remote gives for a refusal; with `DEPLOY_SIGNING_KEY`, it points to a promotion commit that verifies with the key, carries the tree of `main`'s commit, has the previous promotion and that commit as parents, replaces with a warning a tip that is unsigned or a forged promotion rather than building on it or calling it already promoted, and a key that cannot sign moves nothing |
| `feature-auto-upgrade-signed` | always, on `x86_64-linux` | a machine with `signers` refuses an unsigned commit, one signed by another key, one signed for another host, one signed with GPG by a key of root's keyring and one changed after signing, builds nothing and names the source and reason; it applies a correctly signed one, takes it from a mirror when the first source serves an unsigned one, follows a promotion commit whose parent is the running one, refuses an older signed promotion replayed, and follows a new promotion after the deploy branch was reset to an unsigned commit of `main` |
| `feature-auto-upgrade-sources` | always, on `x86_64-linux` | a host whose `mirrors` holds an `http://`, a `git://` or a relative URL fails to evaluate with a message naming each one, while `https://`, `ssh://`, `user@host:path`, `file://` and absolute paths pass |
| `ci-mirror` | always, on `x86_64-linux` | on local git repositories, `main` and every `deploy/<host>` reach the mirror and nothing else does, a deploy branch deleted or moved back in the fork follows on the mirror while other mirror branches stay, a diverged `main` is refused without holding the deploy branches back, and a local http server receives the Basic auth header of `MIRROR_TOKEN`, none without it |
| `ci-report` | always, on `x86_64-linux` | `ci-build` keeps a clean log excerpt and retries a VM test once; `ci-report` turns sample runs into the right verdict, a table per host that never mixes two hosts, the cause of a failure, the boot time beside the base's with `n/a` when one is missing, the screenshot artifacts, a comment cut to fit, Narvi's line above the verdict of a green, a red and an unplanned run, Haiku's summary above the package diff from a stub server, and the same report as without Narvi when that server fails; `ci-build` keeps the boot time and screenshots of a passing check's output |
| `narvi` | always, on `x86_64-linux` | `narvi say` picks the same line for the same seed and several lines over several seeds, prints nothing for an empty, broken or missing list; against a stub server, `summarize` asks GitHub for a token with the Anthropic audience, sends the exchange body with the workspace only when set, then calls Messages with the bearer token alone, uses `ANTHROPIC_API_KEY` without OIDC or after a refused exchange, prints nothing and exits 0 without credentials, on a 500, a malformed or empty answer and a timeout, keeps all its calls within one budget, does not call again for an input that failed in the same job, flattens a hostile answer (an HTML comment, a mention, a link, an image, a URL) to plain text, never prints a token, cuts a large input and reuses a cached summary; with a stand-in for `gh`, `pr` and `release` put the line and the summary on top of a body kept intact below, only once, leave a body alone when there is nothing to say, and exit 0 with a warning when `gh` cannot read or edit; every moment of `messages.json` has 5 to 10 plain lines |
| `ci-badges` | always, on `x86_64-linux` | `ci-badges` turns sample runs into the right badges (every check passing, a failing count in red, cached checks and VM tests counted, the boot time of `example` from the run or the binary cache and none when neither has it, only `checks` when the flake does not evaluate) and a nightly summary into `reproducible`, `N paths differ` or `not finished`; on local git repositories, a publish adds its files to an orphan `badges` branch, keeps the ones it does not write and adds no commit when nothing changed |
| `reproducibility` | always, on `x86_64-linux` | with a stand-in for `nix`, `reproducibility` says an identical rebuild is identical, shows the diffoscope lines of a differing one without failing, and fails only when a system does not build, after trying the other machines |

## Reproducibility

`nix build --rebuild` builds the system derivation of a machine once more and compares the result with the first build, byte for byte. It rebuilds the last derivation only, the one that assembles the system, not every package under it, so it catches what that step adds (timestamps, ordering) rather than the reproducibility of nixpkgs. Published measurements put the share of NixOS builds that rebuild identically between 69 and 91 %, so a difference now and then is expected: the job stays green and its summary shows diffoscope's first 200 lines for each machine that differs. It fails only when a system does not build.

## Live badges

The README badges under "Proved by CI" read [shields.io endpoint](https://shields.io/badges/endpoint-badge) files from the orphan branch `badges`, so they follow every run instead of going stale. On a push to `main`, the `badges` job of `check` runs `nix run .#ci-badges -- checks badges` on the run's results and writes `checks.json` (`36/36 passing`, or the failing count in red), `vm-tests.json` (boot and VM tests), `hosts.json` (hosts built) and `boot-time.json` (the `example` host to `multi-user.target`, or the first host with a boot test; `BADGE_HOST` picks another). The nightly `reproducibility` workflow writes `reproducibility.json` (`reproducible` or `N paths differ`). `nix run .#ci-badges -- publish badges` commits them to `badges` with `GITHUB_TOKEN`, the only place where a job gets `contents: write`. Publishing never turns a check red: a failure is a warning on the step.

A fork gets its own badges once its CI has run on `main`: point the badge URLs at `https://raw.githubusercontent.com/<owner>/<repo>/badges/<file>.json`.

## aarch64 VM tests

`vmCheck` and the boot tests run on `x86_64-linux` only, because they need KVM. Read on 17/10/2026, GitHub's arm64 Linux runner `ubuntu-24.04-arm` does not expose `/dev/kvm`, so they stay there:

- GitHub's runner reference lists the arm64 Linux runners but says nothing about KVM; the only virtualization it mentions is hardware acceleration for the Android SDK on Linux runners, and that nested virtualization is not supported on arm64 macOS runners ([Supported runners and hardware resources](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [Larger runners reference](https://docs.github.com/en/actions/reference/runners/larger-runners)).
- The two announcements do not contradict each other on KVM, they cover different offers and neither mentions KVM: [arm64 hosted runners for public repositories are now generally available](https://github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available/) (07/08/2025, the free standard runners) and [arm64 Linux and Windows runners are now generally available](https://github.blog/changelog/2026-09-03-github-actions-arm64-linux-and-windows-runners-are-now-generally-available/) (its URL says 03/09/2026 while the page is dated 03/09/2024; the paid larger runners, Team and Enterprise Cloud plans only). Between them, [arm64 standard runners are now available in private repositories](https://github.blog/changelog/2026-01-29-arm64-standard-runners-are-now-available-in-private-repositories/) (29/01/2026).
- The request for KVM on these runners, [actions/runner-images#14062](https://github.com/actions/runner-images/issues/14062) (opened 03/12/2025), was closed as not planned on 20/08/2026: a maintainer of the Arm image answered on 05/12/2025 that the runners are virtual machines that do not expose KVM, nested virtualization on AArch64 not being available in Azure yet, and the runner-images maintainers that `/dev/kvm` has to come from the host, that the image cannot add it, and that nested virtualization is not officially supported on GitHub-hosted runners in general.

An `aarch64-linux` VM test would run under emulation (TCG), many times slower than the 60 minute job limit allows for a boot test. Until GitHub ships KVM on arm64, `vmCheck` stays `x86_64-linux` only; a fork with a self-hosted `aarch64-linux` runner exposing `/dev/kvm` can run them there.

## Running the CI scripts locally

The matrix and scripts live in Nix (`modules/flake/ci/`), so they run the same on your machine.

| Command | What it does |
|---|---|
| `nix eval --json .#ci.matrix` | every check with its system, runner and `retry` (true for a NixOS VM test) |
| `nix run .#ci-plan -- pull_request` | plan a run as CI does (`pull_request` or `push`), skipping what your store or substituters hold |
| `nix build -L .#checks.x86_64-linux.example-boots` | build one check |
| `nix run .#ci-build -- x86_64-linux example-boots results --retry` | build one check as CI does: with `--retry`, a failure is rebuilt once and a pass on the second attempt is `unstable`; `--base-input <input> --base <flake-ref>` overrides that input of the check, as CI does for a job whose plan names a `baseInput` (`feature-migration`) with `github:<owner>/<repo>/<base commit>`, and keeps the locked input when the commit is all zeros; writes `results/x86_64-linux-example-boots.json` with the outcome, the attempts, an excerpt of the failing log, after each failed attempt and at the end, and from a passing check's output its `bootTime` (from `boot-time.json`) and `screenshots` (its `.png` files, copied to `results/screenshots/<system>-<check>-<name>.png`) |
| `nix run .#ci-boot-time -- <flake ref> results/base` | read the boot time of every `<host>-boots` of `<flake ref>` (for instance `github:<owner>/<repo>/<sha>`) that the binary cache or your store already holds, without building anything, into `results/base/boot-time.json`, which `ci-report` shows as the base |
| `nix run .#reproducibility [-- <host>...]` | build each host's system (every host of your platform by default), rebuild it with `nix build --rebuild` and print a Markdown summary, with diffoscope's first 200 lines for a difference; exits 1 only when a system does not build |
| `RESULTS_DIR=results HOST_CHECKS=... PLANNED=... SKIPPED=... LINT_RESULT=success nix run .#ci-badges -- checks badges` | write the shields endpoint files of a run's results to `badges/`; `reproducibility <summary.md> badges` writes `reproducibility.json` from the output of `nix run .#reproducibility`; `publish badges` commits the files of `badges/` to the `badges` branch of `origin` |
| `nix eval --json .#ci.hostChecks` | the checks of each host, the exact list the report groups them by |
| `nix eval --json .#ci.descriptions` | what each check proves, as the report says it |
| `HOST_CHECKS=$(nix eval --json .#ci.hostChecks) DESCRIPTIONS=$(nix eval --json .#ci.descriptions) RESULTS_DIR=results nix run .#ci-report` | write the report of the results in `results/` to stdout; `PLANNED`, `SKIPPED`, `HOST_CHECKS` and `DESCRIPTIONS` take the `checks`, `skipped`, `hostChecks` and `descriptions` JSON of `ci-plan`, `CACHE_OUTCOME`, `LINT_RESULT` and `PLAN_RESULT` the outcome of the cache step, the lint job and the plan job, `RUN_URL` the link to the run; `--comment` cuts it to fit a PR comment |
| `HOST_CHECKS=$(nix eval --json .#ci.hostChecks) PLANNED='[...]' SKIPPED='[...]' nix run .#ci-promote -- results <sha>` | move each `deploy/<host>` of `origin` to `<sha>` when every check of that host in `results/` passed; `PLANNED` and `SKIPPED` take the `checks` and `skipped` JSON of `ci-plan`, `DEPLOY_SIGNING_KEY` the private key that signs the promotion commits |
| `MIRROR_TOKEN=<token> nix run .#ci-mirror -- <mirror url>` | push `main` and every `deploy/<host>` as `origin` held them at the last fetch to the mirror, pruning the deploy branches `origin` no longer has; `MIRROR_TOKEN` is optional |
| `nix run .#narvi -- say report-green` | print the line Narvi says for a green run; `nix run .#narvi -- summarize packages closure.md` prints Haiku's summary of an nvd diff when `ANTHROPIC_API_KEY` is set, `ANTHROPIC_BASE_URL` points it at another endpoint |
| `nix run .#closure-diff -- <base rev or branch>` | package diff of the hosts in `nix eval --json .#ci.hosts` for the current platform |
| `nix run .#sync-develop` | merge `main` into `develop` and push, from a checkout of `develop` |
