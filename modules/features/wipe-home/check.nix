{ lib, config, ... }:

let
  inherit (config.helpers) hostsOn hasDisk testRevision;
  inherit (config.features) wipe-home;

  hostConfig = name: config.flake.nixosConfigurations.${name};

  installsKeepingHome = name: hasDisk name && !(hostConfig name).config.impermanence.wipeHome;
in
{
  perSystem =
    { system, ... }:
    {
      checks = lib.concatMapAttrs (name: _: {
        "${name}-wipe-home" =
          ((hostConfig name).extendModules {
            modules = [
              wipe-home
              testRevision
            ];
          }).config.system.build.installTest;
      }) (lib.filterAttrs (name: _: installsKeepingHome name) (hostsOn system));
    };
}
