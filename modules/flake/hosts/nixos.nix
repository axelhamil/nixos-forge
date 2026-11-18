{ lib, config, ... }:

let
  isValidName = name: builtins.match "[A-Za-z0-9]([A-Za-z0-9_-]{0,61}[A-Za-z0-9])?" name != null;
in
{
  flake.nixosConfigurations = lib.mapAttrs (
    name: host:
    lib.throwIfNot (isValidName name)
      "configurations.nixos.${name}: a host name has 1 to 63 letters, digits, - and _, and starts and ends with a letter or digit"
      (config.helpers.registryMachine host [ ])
  ) config.configurations.nixos;
}
