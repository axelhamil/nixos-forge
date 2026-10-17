{ lib, config, ... }:

let
  namespaces = {
    base.base = config.base;
    inherit (config) profiles features hardware;
  };

  owners = name: lib.attrNames (lib.filterAttrs (_: namespace: namespace ? ${name}) namespaces);

  shared = lib.filter (name: lib.length (owners name) > 1) (
    lib.concatMap lib.attrNames (lib.attrValues namespaces)
  );
in
{
  flake.modules.nixos = lib.throwIf (shared != [ ]) "flake.modules.nixos: ${
    lib.concatMapStringsSep ", " (
      name: "\"${name}\" is in ${lib.concatStringsSep " and " (owners name)}"
    ) (lib.unique shared)
  }; a name is published by one namespace only." (lib.mergeAttrsList (lib.attrValues namespaces));
}
