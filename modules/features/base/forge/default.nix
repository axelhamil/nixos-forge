{ config, withSystem, ... }:

let
  inherit (config.helpers) wrapForge;
in
{
  base =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      forge = withSystem pkgs.stdenv.hostPlatform.system ({ config, ... }: config.packages.forge);
    in
    {
      options.forge.commands = lib.mkOption {
        type = lib.types.listOf lib.types.package;
        default = [ ];
        example = lib.literalExpression "[ pkgs.forge-backup ]";
        description = "Packages whose `bin/forge-<name>` programs run as `forge <name>`, found through `FORGE_COMMANDS_PATH` before `PATH`, without rebuilding forge. Each one follows the contract of docs/reference/forge.md.";
      };

      options.forge.runtimeInputs = lib.mkOption {
        type = lib.types.listOf lib.types.package;
        default = [ ];
        internal = true;
        description = "Packages whose `bin` the built-in commands of `forge` find on `PATH`, before the caller's own `PATH`. The modules behind those commands add to the list.";
      };

      options.forge.environment = lib.mkOption {
        type = lib.types.attrsOf lib.types.str;
        default = { };
        internal = true;
        description = "Variables the built-in commands of `forge` read, set unless the caller already sets them. They belong to the system `forge` comes from, so a specialisation's `forge` reads its own.";
      };

      options.forge.package = lib.mkOption {
        type = lib.types.package;
        readOnly = true;
        internal = true;
        description = "The `forge` this system installs, wrapped with its commands, runtime inputs and environment; services run it so they read the same configuration as the command line.";
      };

      config.forge.package = wrapForge pkgs forge {
        inherit (config.forge) commands runtimeInputs environment;
      };

      config.environment.systemPackages = [ config.forge.package ];
    };
}
