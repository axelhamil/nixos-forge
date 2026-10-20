# CI of the starter

What CI proves about the starter's own examples and upgrade path. A fork that ran `nix run .#init` has no example host left, so these checks either do not exist there or prove nothing it relies on. The CI a fork runs is in [ci.md](../how-to/ci.md).

## Checks of the examples

| Check | When | What it proves |
|---|---|---|
| `feature-migration` | while `example` exists and imports `disk` here and on the base, on `x86_64-linux`; built on every run, never skipped as cached | the `example` host of the base installs on a disk and writes state in `/persist`, `/var/lib/nixos` and a home; this version's `example` is installed with `switch-to-configuration boot` and started; the machine-id, the user's uid and every file are still there (a home that this version starts wiping is found in the wiped home's snapshot, as [wipe-home.md](../how-to/wipe-home.md#turn-it-on-for-a-home-in-use) says), `doctor` loses no file and sees no failed unit but the `home-manager-<user>` services, which never start on the store the test shares over 9p, and the `systemd-boot-check-no-failures` they fail; Secure Boot and TPM2 are left to `example-disk`. The base is the `migration-base` input: in a PR, `ci-build --base` overrides it with the PR's base commit, on a push with the commit before the push (a push that creates the branch has none and keeps the locked input), in the repository the run belongs to; locally it is the starter's `main` as locked. `init` leaves the input in a fork, since CI overrides it there |
| `feature-migration-without-disk` | always, on `x86_64-linux` | a minimal machine without `disk`, as this version or as the base, gets no `feature-migration` and the flake still evaluates, so an `example` without `disk` keeps `nix flake check` green |
| `init` | while the `example` host exists | `nix run .#init -- --yes` on a copy of the flake writes a host and a person that parse, with the answers and features asked, removes the examples, renames the example person in the hosts it keeps, refuses to overwrite a host or a person, writes the same new deploy signing key in the hosts it writes and keeps, saves the private key in a 0600 file of a 0700 folder under `XDG_RUNTIME_DIR` and prints only its path in the `gh secret set` and `shred -u` command, evaluates the `toplevel` of every host left with the flake's locked inputs, rewrites the README's links to the examples to the host and person it wrote, and runs `check-docs` on the result, as the `docs` check does; an unknown package in `--apps`, a person field of the wrong type and an unknown option in a person's `module` make it fail |
| `example-hosts` | while an example host exists, on `x86_64-linux` | every example host has a line, enabled or commented out, for every registry field, profile, feature, hardware module and app of the starter, and the person the `example` host names has one for every person field |

## Upgrades from the previous release

Every change is also tried as an upgrade: `feature-migration` installs the `example` host of the base revision, writes state, installs this version for the next boot and checks that the state is still there once it started. In a fork without the `example` host, or whose `example` here or on the base does not import `disk`, the check does not exist.

## Releases that edit the examples

A release editing `modules/hosts/example*` or `modules/users/alice` gives every fork that removed them a modify/delete conflict on `nix run .#upstream`, which it settles by keeping them deleted. Keep those edits to what a new field or module needs, so the diff a fork reads shows what the new field looks like.

## Badges and boot time of the starter

`ci-badges` reads the boot time of `BADGE_HOST`, `example` by default, and falls back to the first host with a boot test. For reference, `example` measured 7.0 s in total (0.4 s kernel, 3.6 s initrd, 3.0 s userspace) on a local machine on 17/10/2026.
