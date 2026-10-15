{ lib, config, ... }:

{
  helpers.hostsOn =
    system: lib.filterAttrs (_: host: host.system == system) config.configurations.nixos;
}
