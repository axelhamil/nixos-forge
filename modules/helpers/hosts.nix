{ lib, config, ... }:

{
  helpers.hostsOn =
    system: lib.filterAttrs (_: host: host.system == system) config.configurations.nixos;

  helpers.hasDisk = name: config.flake.nixosConfigurations.${name}.config.system.build ? installTest;
}
