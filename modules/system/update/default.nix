{
  flake.modules.nixos.base =
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
      options.update.flake = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "/home/alice/nixos";
        description = "Flake the machine updates from: a git checkout on the machine, or a flake reference such as `github:alice/nixos`.";
      };

      config = lib.mkIf (flake != null) {
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
      };
    };
}
