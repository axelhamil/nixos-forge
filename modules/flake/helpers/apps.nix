{ lib, config, ... }:

{
  helpers.appPart = name: part: {
    key = "nixos-forge/apps/${name}/${part}";
    _class = part;
    imports = lib.optional (config.apps.${name}.${part} != null) config.apps.${name}.${part};
  };
}
