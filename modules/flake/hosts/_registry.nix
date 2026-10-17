{
  lib,
  modules,
  names,
}:

let
  empty = list: lib.genAttrs list (_: { });

  flake = lib.evalModules {
    specialArgs.inputs = {
      nixos-hardware.nixosModules = { };
      nixpkgs.legacyPackages = { };
    };
    modules = modules ++ [
      {
        options.systems = lib.mkOption { default = [ "x86_64-linux" ]; };
        config = lib.mapAttrs (_: empty) names;
      }
    ];
  };

  host =
    hostName:
    let
      nixos = lib.evalModules {
        specialArgs.pkgs = { };
        modules = [
          flake.config.configurations.nixos.${hostName}.module
          {
            options.environment.systemPackages = lib.mkOption { default = [ ]; };
            config._module.check = false;
          }
        ];
      };
    in
    {
      options = builtins.attrNames nixos.config;
      packages = nixos.config.environment.systemPackages;
    };
in
{
  hosts = builtins.attrNames flake.config.configurations.nixos;
  inherit host;
}
