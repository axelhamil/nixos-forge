{ config, ... }:

let
  inherit (config.helpers) flakePackage wrapForge;
in
{
  base =
    {
      lib,
      pkgs,
      config,
      options,
      ...
    }:
    let
      enrollConfig = pkgs.writeText "enroll.json" (
        builtins.toJSON {
          inherit (config.forge) luksDevice;
          inherit (config.forge.enroll) volumeKeyDescription pcrs;
        }
      );

      forge = flakePackage pkgs "forge";
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

      options.forge.luksDevice = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default =
          if options ? disko then
            lib.attrByPath [
              "disko"
              "devices"
              "disk"
              "main"
              "content"
              "partitions"
              "luks"
              "device"
            ] null config
          else
            null;
        readOnly = true;
        internal = true;
        description = "LUKS partition of the `disk` feature, null without it. `forge doctor`, `forge enroll` and the `tpm2-enroll` service read it.";
      };

      options.forge.enroll = {
        volumeKeyDescription = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          internal = true;
          description = "Description of the volume key the initrd links in the user keyring for `forge enroll tpm2`, null when the host does not enroll a TPM2.";
        };

        pcrs = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          internal = true;
          description = "PCRs `forge enroll tpm2` binds the TPM2 token to, in the syntax of `systemd-cryptenroll --tpm2-pcrs`.";
        };
      };

      config.forge.environment.FORGE_ENROLL_CONFIG = "${enrollConfig}";

      config.forge.package = wrapForge pkgs forge {
        inherit (config.forge) commands runtimeInputs environment;
      };

      config.environment.systemPackages = [ config.forge.package ];
    };
}
