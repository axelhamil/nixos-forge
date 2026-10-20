---
name: extend-flake
description: Recipes to add an app, feature, hardware module, profile, base tool, host, person, helper, flake package, CI script or flake input to this NixOS flake, with the checks, example host lines and docs each one needs. Use before creating or renaming anything under modules/ or touching flake.nix inputs.
---

# Extend the flake

`AGENTS.md` holds the architecture and the rules; this skill holds the steps. Every recipe ends with "Finish" below.

## Rules for every recipe

- One folder per tool under its layer (`modules/apps/<name>/`, `modules/features/<name>/`...), never a loose `.nix` at layer level. A path segment starting with `_` is not imported: use it for files a neighbour imports by hand.
- `git add` every new file before any `nix` command, or nix does not see it.
- No comments in `.nix` and `.sh`, except the example hosts and `modules/users/alice`.
- Values a user may override go through `helpers.mkDefaults`. `enable` flags and lists meant to be extended stay plain.
- Unfree package: add its `lib.getName` to `nixpkgs.config.allowUnfreePackages`, never `allowUnfree = true`.
- State outside `/nix`, `/home`, `/persist` is wiped at boot: persist it from the module that owns it (`environment.persistence."/persist".directories`), or list it in `doctor.ephemeralPaths` when it may vanish. Home state: `home.persistence."/persist".directories` with `helpers.relativeToHome`, or `helpers.persistInEveryHome` from a NixOS module.
- No module imports an app or a feature. Read a neighbour's option instead (`config.programs.fuzzel.enable`).
- Never set `nixpkgs.hostPlatform`: the registry does.

## App: `modules/apps/<name>/`

Everything a user runs, with its config. Tools with nothing to configure are nixpkgs names in `apps`, not apps.

```nix
{ config, ... }:
let
  inherit (config.helpers) mkDefaults relativeToHome;
in
{
  apps.<name>.nixos.nixpkgs.config.allowUnfreePackages = [ "<name>" ];
  apps.<name>.homeManager =
    { config, ... }:
    {
      programs.<name> = { enable = true; settings = mkDefaults { }; };
      home.persistence."/persist".directories = [ (relativeToHome config config.programs.<name>.configDir) ];
    };
}
```

- Two parts, each optional: `nixos` (a NixOS module, on the host) and `homeManager` (a home-manager module, for every user). A home-manager option in the `nixos` part throws.
- Smallest real one: `modules/apps/btop/default.nix`. With persistence and a test: `modules/apps/firefox/`.
- Test: `check.nix` with `{ config, ... }: { perSystem = config.helpers.appCheck "<name>" { testScripts = [ ./test.py ]; }; }`, giving `app-<name>`. The machine has user `tester`; `test.py` starts with `machine.wait_for_unit("home-manager-tester.service")`.
- Docs: the table and text of `docs/apps.md`.

## Feature: `modules/features/<name>/`

```nix
{
  features.<name> = {
    programs.nix-ld.enable = true;
  };
}
```

- Real ones: `nix-ld` (smallest), `printing` (`doctor.ephemeralPaths`), `docker` (home state through `persistInEveryHome`, reached with an outer `{ config, ... }:` and an inner NixOS module).
- Test: `config.helpers.featureCheck "<name>" { machine = { }; testScripts = [ ./test.py ]; }`, giving `feature-<name>`. Persistence in the test: `machine.environment.persistence."/persist".enable = true;`.
- Docs: the README feature table and the matching `docs/` page (`services.md`, `dev.md`...).

## Hardware: `modules/features/hardware/<name>/`

- `{ hardware.<name> = { ... }; }`. One file may publish several names (`raspberry-pi`).
- A name in `hardware = [ ]` resolves to the starter's `hardware.<name>`, else to `inputs.nixos-hardware.nixosModules.<name>`: add a module only for what nixos-hardware lacks.
- No module check: the hosts' `<host>-toplevel` covers it.

## Profile: `modules/features/profiles/<name>/`

- `{ profiles.<name> = { ... }; }`, one role per host (`profile = "<name>";`).
- Test: `config.helpers.profileCheck "<name>" { testScripts = [ ./test.py ]; }`, giving `profile-<name>`.
- Name it in the comment of the `profile` field of every example host, in the `profile` description of `modules/flake/hosts/options.nix`, then run `nix run .#options-doc`.

## Base tool: `modules/features/base/<tool>/`

- `{ base = { ... }; }`: on every host, merged with the other base tools. Think twice: a feature is opt-in, base is not.
- Test: `config.helpers.baseCheck "<tool>" { testScripts = [ ./test.py ]; }`, giving `base-<tool>`.

## Host: `modules/hosts/<name>/default.nix`

```nix
{
  configurations.nixos.<name> = {
    system = "x86_64-linux";
    stateVersion = "26.05";
    profile = "desktop";
    features = [ "disk" "ssh" ];
    apps = [ "zsh" "git" ];
    hardware = [ ];
    users.<person> = { };
  };
}
```

- Every field with its type: `docs/options.md`. Name: 1 to 63 letters, digits, `-`, `_`.
- Copy `modules/hosts/example` rather than writing from scratch; a fresh fork runs `nix run .#init` instead.
- Optional: `installer.testedHosts = [ "<name>" ];` adds `<name>-installer`. Checks `<name>-toplevel`, `<name>-boots` and the disk ones are generated.
- A variant: `specialisations.gaming = { features = [ ]; apps = [ "steam" "gamemode" "mangohud" ]; hardware = [ ]; module = { }; };` adds to the host's lists, by the same names; `update -s gaming` switches to it, `update -S` goes back ([docs/specialisations.md](../../../docs/specialisations.md)). A variant that changes the kernel or a kernel module package applies from the boot menu only.

## Person: `modules/users/<name>/default.nix`

- `{ users.<name> = { fullName = ""; email = null; sshKeys = [ ]; apps = [ ]; module = { }; }; }`. `module` is the person's home-manager config; several files of the folder may each set part of it.
- A host gets the account with `users.<name> = { admin = true; groups = [ ]; apps = [ ]; };`.

## Registry field: `modules/flake/hosts/options.nix` or `users.nix`

- Plain values only, names as lists, never `enable` booleans; turn them into imports at the flake-parts level, never inside a NixOS module.
- Add a line for the field in all three example hosts (or in `modules/users/alice`), run `nix run .#options-doc`, and keep `init.sh` able to rewrite the host by its lines.
- A field that names modules resolves inside `selection` in `options.nix`, so a host's `specialisations` take it too; its unknown-name message names the field path (`specialisations.<variant>.<field>` for a variant) and gets a case in `modules/flake/hosts/namespaces-check.nix`.

## Helper: `modules/flake/helpers/<topic>.nix`

- `{ lib, ... }: { helpers.<name> = ...; }`, read with `inherit (config.helpers) <name>;`. One file per topic.

## Flake package or CI script

- Folder `modules/flake/<tool>/` (CI: `modules/flake/ci/`) with `<tool>.sh` and a `default.nix`:

```nix
{
  perSystem =
    { pkgs, ... }:
    {
      packages.<tool> = pkgs.writeShellApplication {
        name = "<tool>";
        runtimeInputs = [ pkgs.git ];
        text = builtins.readFile ./<tool>.sh;
      };
    };
}
```

- Run with `nix run .#<tool>`. Test: `check.nix` plus `<tool>-test.sh`, as `modules/flake/upstream/`.
- A workflow only calls `- run: nix run .#<tool>`; never a multi-line script in YAML. Text for people goes through `nix run .#narvi`. Document it in `docs/ci.md`.

## Flake input: `flake.nix`

- `<name> = { url = "github:owner/repo"; inputs.nixpkgs.follows = "nixpkgs"; };`, then read it as `{ inputs, ... }:` in the module that uses it (`modules/apps/nix-index`).
- `nix flake lock`, then `git add flake.nix flake.lock`.

## Finish

1. Every new profile, feature, hardware module or app: one line in each of `modules/hosts/example`, `example-server` and `example-pi`, in its list, as `"<name>" # what the forker gets (docs/<page>.md)`, commented out (`# "<name>" # ...`) where it does not fit the role. A commented line changes nothing in the build. The `example-hosts` check names any missing line.
2. `ROADMAP.md` when the change ships a lot, with its guardrail check.
3. `nix fmt`, then `nix flake check --no-build`; build the new check: `nix build -L .#checks.x86_64-linux.<check>`.
4. Show the staged diff and wait for the maintainer's go before committing.
