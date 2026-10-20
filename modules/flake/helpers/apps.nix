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
  helpers.appPart = name: part: {
    key = "nixos-forge/apps/${name}/${part}";
    _class = part;
    imports = lib.optional (config.apps.${name}.${part} != null) config.apps.${name}.${part};
  };

  helpers.appMachine =
    name: hostFields: personApps:
    let
      host = lib.evalModules {
        modules = hostType.getSubModules ++ [
          {
            _module.args.name = name;
            stateVersion = "26.05";
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
        ];
      };
    in
    (inputs.nixpkgs.lib.nixosSystem {
      modules = [
        host.config.module
        { nixpkgs.hostPlatform = host.config.system; }
      ];
    }).config;
}
