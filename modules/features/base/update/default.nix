{ self, ... }:

{
  base =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      inherit (config.update) flake;
    in
    {
      options.update = {
        flake = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          example = "/home/alice/nixos";
          description = "Flake the machine updates from: a git checkout on the machine, or a flake reference such as `github:alice/nixos`.";
        };

        mirrors = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          example = [ "https://codeberg.org/alice/nixos.git" ];
          description = "Extra git URLs of the fork, tried in order when the first one is unreachable: `https://`, `ssh://`, an scp-like `user@host:path`, `file://` or an absolute path. `http://` and `git://` are refused, since they are not encrypted.";
        };
      };

      options.autoUpgrade = {
        operation = lib.mkOption {
          type = lib.types.enum [
            "boot"
            "switch"
          ];
          default = "boot";
          description = "How `auto-upgrade` applies a new version: `boot` makes it the default for the next boot, `switch` also starts it now.";
        };

        allowLocalBuild = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Whether `auto-upgrade` may build what the binary cache lacks on the machine itself; when false it waits for the cache.";
        };

        rebuildCommand = lib.mkOption {
          type = lib.types.str;
          default = lib.getExe config.system.build.nixos-rebuild;
          defaultText = lib.literalExpression "lib.getExe config.system.build.nixos-rebuild";
          internal = true;
          description = "Command `auto-upgrade` runs to build and apply a version.";
        };
      };

      config = lib.mkMerge [
        { system.configurationRevision = lib.mkDefault (self.rev or self.dirtyRev or null); }

        (lib.mkIf (flake != null) {
          programs.nh.flake = flake;

          environment.systemPackages = [
            (pkgs.writeShellApplication {
              name = "update";
              runtimeInputs = [
                pkgs.git
                config.programs.nh.package
              ];
              runtimeEnv.FLAKE = flake;
              text = builtins.readFile ./update.sh;
            })
          ];
        })
      ];
    };
}
