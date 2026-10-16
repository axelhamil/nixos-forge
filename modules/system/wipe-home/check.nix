{ lib, config, ... }:

let
  inherit (config.helpers) hostsOn;
  inherit (config.flake.modules.nixos) wipe-home;

  hostConfig = name: config.flake.nixosConfigurations.${name};

  installsKeepingHome =
    name:
    let
      nixos = (hostConfig name).config;
    in
    nixos.system.build ? installTest && !nixos.impermanence.wipeHome;
in
{
  perSystem =
    { system, ... }:
    {
      checks = lib.concatMapAttrs (name: _: {
        "${name}-wipe-home" =
          ((hostConfig name).extendModules { modules = [ wipe-home ]; }).config.system.build.installTest;
      }) (lib.filterAttrs (name: _: installsKeepingHome name) (hostsOn system));
    };
}
