{
  lib,
  inputs,
  config,
  ...
}:

let
  isValidName = name: builtins.match "[A-Za-z0-9_-]+" name != null;
in
{
  flake.nixosConfigurations = lib.mapAttrs (
    name: host:
    lib.throwIfNot (isValidName name)
      "configurations.nixos.${name}: a host name may only contain letters, digits, - and _"
      (
        inputs.nixpkgs.lib.nixosSystem {
          modules = [
            host.module
            { nixpkgs.hostPlatform = host.system; }
          ];
        }
      )
  ) config.configurations.nixos;
}
