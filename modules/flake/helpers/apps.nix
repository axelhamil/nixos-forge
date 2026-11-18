{ lib, config, ... }:

let
  inherit (config.helpers) registryHost registryMachine;
in
{
  helpers.appPart = name: part: {
    key = "nixos-forge/apps/${name}/${part}";
    _class = part;
    imports = lib.optional (config.apps.${name}.${part} != null) config.apps.${name}.${part};
  };

  helpers.appMachine =
    name: hostFields: personApps:
    (registryMachine (registryHost name [
      {
        users.${lib.head (lib.attrNames config.users)}.apps = personApps;
        module = {
          fileSystems."/" = lib.mkDefault {
            device = "/dev/disk/by-label/${name}";
            fsType = "ext4";
          };
          boot.loader.grub.enable = false;
        };
      }
      hostFields
    ]) [ ]).config;
}
