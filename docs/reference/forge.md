# forge

`forge` is the command line of your flake. It is on every host, and in the flake as `nix run .#forge -- <command>`. Its built-in commands are written in Rust, in the Cargo workspace `crates/`; the commands your fork adds are any programs you list, in any language, without rebuilding `forge`.

## Built-in commands

| Command | What it does |
|---|---|
| `forge auto-upgrade` | what the `auto-upgrade` service runs: fetches `deploy/<host>` from the host's sources, checks it (signature, history, the generation that did not start) and applies it with `nixos-rebuild`, then writes `/var/lib/auto-upgrade/state.json` and a row of `history.tsv` and prints `<outcome>: <message>` ([updates](../explanation/updates.md#a-machine-upgrades-on-its-own)). It reads the host's settings from `FORGE_AUTO_UPGRADE_CONFIG`, set only on a host with `auto-upgrade`, and exits 1 only when the outcome is `failed` |
| `forge check-docs [ROOT]` | checks the docs of the flake at `ROOT` (the current directory by default), as the `docs` check does: every relative link and `#anchor` of `README.md` and `docs/` resolves, every page of `docs/` is in `docs/README.md`, no page outside `docs/maintenance/` names upstream's examples or repository, and every app, feature, hardware module and profile has a heading or first table cell naming it in backticks. Violations listed in `modules/flake/docs/pending.txt` pass; a listed one that no longer fails is an error |
| `forge doctor [--all \| --history]` | checks how the machine is doing, then runs the doctor checks of the commands your fork adds ([doctor](../how-to/doctor.md)). `--all` lists every file the next boot erases, `--history` prints the last 10 runs of `auto-upgrade` instead of the checks. It reads the host's settings from `FORGE_DOCTOR_CONFIG`, which the wrapper of each system sets, else `/etc/forge/doctor.json`, and exits 1 when a check fails |
| `forge enroll fido2 [DEVICE]` | enrolls a FIDO2 security key on the LUKS volume `DEVICE`, the disk of the host's `disk` feature by default, with `systemd-cryptenroll --fido2-device=auto`; the passphrase stays as a fallback ([fido2](../how-to/fido2.md#unlock-the-disk-with-the-key)). It reads the disk from `FORGE_ENROLL_CONFIG` |
| `forge enroll tpm2` | what the `tpm2-enroll` service runs at boot on a host with `disk` and `secure-boot`: once Secure Boot is enforced, it enrolls a TPM2 token bound to it with the volume key the initrd left in the keyring, then drops that key ([install](../tutorials/install.md#step-6-check-the-tpm2-unlocks-the-disk)). It refuses to run outside the service |
| `forge install` | the installer ISO's assistant, started on its first console: picks the disk, asks the passphrase and the passwords, partitions with the host's disko layout and installs the prebuilt system; `--disk`, `--passphrase-file`, `--password-file USER=FILE`, `--beside-windows`, `--yes` and `--no-reboot` script it ([install](../tutorials/install.md#step-4-run-the-installer)) |
| `forge update [ARGS...]` | pulls the host's `flake` checkout with `git pull --ff-only`, or refetches its flake reference, then runs `nh os switch ARGS...`; every argument, `--help` included, goes to nh ([updates](../explanation/updates.md)). It reads the flake from `FORGE_UPDATE_FLAKE`, set on a host with a `flake` field |
| `forge help`, `forge --help` | lists the built-in commands, then the commands found on the machine with their summary |

In `check-docs`, a root-relative link (`/README.md`) resolves from the repository root, and a fragment on a folder link (`../reference/#x`) must be an anchor of that folder's `README.md`.

A built-in command always wins: a `forge-check-docs`, `forge-doctor`, `forge-enroll`, `forge-install`, `forge-update` or `forge-auto-upgrade` program is never run.

## Commands a fork adds

`forge <name> [args...]` runs the first executable `forge-<name>` found in the directories of `FORGE_COMMANDS_PATH` (separated by `:`), then of `PATH`, as git and cargo do. The arguments pass through untouched, and `forge` exits with the command's exit code.

On a host, list the packages that hold such programs in `forge.commands`, in the host's `module` or in any module of your fork:

```nix
{ pkgs, ... }:
{
  forge.commands = [
    (pkgs.writeShellApplication {
      name = "forge-hello";
      text = ''
        if [ "''${1:-}" = --forge-describe ]; then
          echo '{"contract":1,"name":"hello","summary":"Greets whoever is named"}'
          exit 0
        fi
        echo "hello $*"
      '';
    })
  ];
}
```

`forge` is then installed wrapped so that `FORGE_COMMANDS_PATH` starts with the `bin` of every package of the list; `forge hello world` prints `hello world`. The wrapper also puts the tools of the built-in commands (coreutils, util-linux, git, systemd, nh, nix, nvd and openssh where `update` and `auto-upgrade` need them, keyutils and cryptsetup where `enroll tpm2` does) before `PATH` and sets `FORGE_DOCTOR_CONFIG`, `FORGE_ENROLL_CONFIG`, `FORGE_UPDATE_FLAKE` and `FORGE_AUTO_UPGRADE_CONFIG` to the host's settings, so a command a fork adds inherits both.

## The command contract

A command follows contract `1`, the only version this `forge` speaks:

- `forge-<name> --forge-describe` prints one JSON object on stdout and exits 0: `{"contract":1,"name":"<name>","summary":"<one line>"}`. `forge` runs it before every call and for `forge --help`, which shows the summary. The object may add `"doctor":true` to offer doctor checks.
- `forge` sets `FORGE_CONTRACT=1` in the command's environment, for the describe call and the real one.
- A command whose description declares another `contract` is refused before it runs, with a message naming its version and the one `forge` speaks. A description that fails or is not that JSON object refuses the command the same way.
- An unknown name fails with exit code 1 and names the command and where it was looked for.

A new contract version comes with a new `forge` release and its notes; a command checks `FORGE_CONTRACT` to support several.

## The doctor checks of a command

`forge doctor`, after its own checks, looks for commands as `forge <name>` does, sorted by name, the first one of each name only, built-ins left out. It describes each one and calls those whose description has `"doctor":true`:

- `forge-<name> --forge-doctor` runs with `FORGE_CONTRACT=1` and `FORGE_DOCTOR_ROOT` set to `1` when `forge doctor` runs as root, `0` otherwise.
- It prints one JSON object per line on stdout: `{"status":"OK","check":"<name>","detail":"<one line>","hint":"<what to do>"}`. `status` is `OK`, `WARN`, `FAIL` or `SKIP`, `check` has 1 to 14 characters, and `hint` is optional. Each line is reported and counted like a built-in check.
- It is stopped, with every process it started, after 30 seconds.
- A command that exits non-zero, prints nothing or prints a line that is not such an object gets one more line, `FAIL` named `forge-<name>`, saying why; the valid lines it printed before it are kept. A command stopped at 30 seconds, or that prints more than 1 MiB on stdout or stderr, gets that `FAIL` line alone.
- A command whose description fails is left out.
