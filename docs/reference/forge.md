# forge

`forge` is the command line of your flake. It is on every host, and in the flake as `nix run .#forge -- <command>`. Its built-in commands are written in Rust, in the Cargo workspace `crates/`; the commands your fork adds are any programs you list, in any language, without rebuilding `forge`.

## Built-in commands

| Command | What it does |
|---|---|
| `forge check-docs [ROOT]` | checks the docs of the flake at `ROOT` (the current directory by default), as the `docs` check does: every relative link and `#anchor` of `README.md` and `docs/` resolves, every page of `docs/` is in `docs/README.md`, no page outside `docs/maintenance/` names upstream's examples or repository, and every app, feature, hardware module and profile has a heading or first table cell naming it in backticks. Violations listed in `modules/flake/docs/pending.txt` pass; a listed one that no longer fails is an error |
| `forge help`, `forge --help` | lists the built-in commands, then the commands found on the machine with their summary |

In `check-docs`, a root-relative link (`/README.md`) resolves from the repository root, and a fragment on a folder link (`../reference/#x`) must be an anchor of that folder's `README.md`.

A built-in command always wins: a `forge-check-docs` program is never run.

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

`forge` is then installed wrapped so that `FORGE_COMMANDS_PATH` starts with the `bin` of every package of the list; `forge hello world` prints `hello world`.

## The command contract

A command follows contract `1`, the only version this `forge` speaks:

- `forge-<name> --forge-describe` prints one JSON object on stdout and exits 0: `{"contract":1,"name":"<name>","summary":"<one line>"}`. `forge` runs it before every call and for `forge --help`, which shows the summary.
- `forge` sets `FORGE_CONTRACT=1` in the command's environment, for the describe call and the real one.
- A command whose description declares another `contract` is refused before it runs, with a message naming its version and the one `forge` speaks. A description that fails or is not that JSON object refuses the command the same way.
- An unknown name fails with exit code 1 and names the command and where it was looked for.

A new contract version comes with a new `forge` release and its notes; a command checks `FORGE_CONTRACT` to support several.
