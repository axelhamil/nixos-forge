{ config, ... }:

{
  helpers.appPart = name: part: {
    key = "nixos-starter/apps/${name}/${part}";
    imports = [ config.apps.${name}.${part} ];
  };
}
