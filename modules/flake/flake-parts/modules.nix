{ lib, config, ... }:

let
  inherit (config.helpers) appPart;

  appParts =
    part:
    lib.mapAttrs (name: _: appPart name part) (
      lib.filterAttrs (_: app: app.${part} != null) config.apps
    );

  namespaces = {
    base.base = config.base;
    inherit (config) profiles features hardware;
    apps = appParts "nixos";
  };

  owners = name: lib.attrNames (lib.filterAttrs (_: namespace: namespace ? ${name}) namespaces);

  shared = lib.filter (name: lib.length (owners name) > 1) (
    lib.concatMap lib.attrNames (lib.attrValues namespaces)
  );
in
{
  flake.modules = {
    nixos = lib.throwIf (shared != [ ]) "flake.modules.nixos: ${
      lib.concatMapStringsSep ", " (
        name: "\"${name}\" is in ${lib.concatStringsSep " and " (owners name)}"
      ) (lib.unique shared)
    }; a name is published by one namespace only." (lib.mergeAttrsList (lib.attrValues namespaces));

    homeManager = appParts "homeManager";
  };

  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks.flake-modules = pkgs.writeText "flake-modules" (
        lib.concatLines (lib.attrNames config.flake.modules.nixos)
      );
    };
}
