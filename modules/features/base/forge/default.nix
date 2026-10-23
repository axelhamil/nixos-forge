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

      withCommands = pkgs.symlinkJoin {
        name = "forge";
        paths = [ forge ];
        nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
        postBuild = ''
          wrapProgram $out/bin/forge --prefix FORGE_COMMANDS_PATH : ${lib.makeBinPath config.forge.commands}
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

      config.environment.systemPackages = [
        (if config.forge.commands == [ ] then forge else withCommands)
      ];
    };
}
