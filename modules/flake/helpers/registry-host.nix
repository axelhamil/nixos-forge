{
  lib,
  inputs,
  config,
  options,
  ...
}:

let
  hostType = options.configurations.nixos.type.nestedTypes.elemType;
in
{
  helpers.registryHost =
    name: modules:
    (lib.evalModules {
      modules =
        hostType.getSubModules
        ++ [
          {
            _module.args.name = name;
            stateVersion = config.helpers.testStateVersion;
          }
        ]
        ++ modules;
    }).config;

  helpers.registryMachine =
    host: modules:
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        host.module
        { nixpkgs.hostPlatform = host.system; }
      ]
      ++ modules;
    };
}
