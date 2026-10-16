{ lib, config, ... }:

let
  inherit (config.helpers) hostsOn;
  inherit (config.flake.modules.nixos) nvidia;

  withoutNvidia = name: !config.flake.nixosConfigurations.${name}.config.hardware.nvidia.enabled;

  nvidiaModuleOf =
    name:
    let
      system = (config.flake.nixosConfigurations.${name}.extendModules { modules = [ nvidia ]; }).config;
    in
    builtins.seq system.system.build.toplevel.drvPath system.hardware.nvidia.package.open;
in
{
  perSystem =
    { system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks = lib.concatMapAttrs (name: _: {
        "${name}-nvidia" = nvidiaModuleOf name;
      }) (lib.filterAttrs (name: _: withoutNvidia name) (hostsOn system));
    };
}
