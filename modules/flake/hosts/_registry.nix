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

  evalNixos =
    module:
    lib.evalModules {
      specialArgs.pkgs = { };
      modules = [
        module
        {
          options.environment.systemPackages = lib.mkOption { default = [ ]; };
          options.specialisation = lib.mkOption {
            type = lib.types.attrsOf lib.types.raw;
            default = { };
          };
          config._module.check = false;
        }
      ];
    };

  host =
    hostName:
    let
      nixos = evalNixos flake.config.configurations.nixos.${hostName}.module;
    in
    {
      options = builtins.attrNames nixos.config;
      packages = nixos.config.environment.systemPackages;
      specialisations = lib.mapAttrs (
        _: variant: builtins.attrNames (evalNixos variant.configuration).config
      ) nixos.config.specialisation;
    };
in
{
  hosts = builtins.attrNames flake.config.configurations.nixos;
  inherit host;
}
