{ withSystem, ... }:

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

      wrapperArgs =
        lib.optionals (config.forge.commands != [ ]) [
          "--prefix"
          "FORGE_COMMANDS_PATH"
          ":"
          (lib.makeBinPath config.forge.commands)
        ]
        ++ lib.optionals (config.forge.runtimeInputs != [ ]) [
          "--prefix"
          "PATH"
          ":"
          (lib.makeBinPath config.forge.runtimeInputs)
        ]
        ++ lib.concatLists (
          lib.mapAttrsToList (name: value: [
            "--set-default"
            name
            value
          ]) config.forge.environment
        );

      wrapped = pkgs.symlinkJoin {
        name = "forge";
        paths = [ forge ];
        nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
        postBuild = ''
          wrapProgram $out/bin/forge ${lib.escapeShellArgs wrapperArgs}
        '';
        meta.mainProgram = "forge";
      };
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

      config.environment.systemPackages = [
        (if wrapperArgs == [ ] then forge else wrapped)
      ];
    };
}
