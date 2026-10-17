{ lib, ... }:

let
  inherit (lib) mkOption types;

  published = field: name: module: {
    key = "nixos-starter/${field}/${name}";
    _class = "nixos";
    imports = [ module ];
  };

  nixosNamespace =
    field: description:
    mkOption {
      type = types.lazyAttrsOf types.deferredModule;
      default = { };
      apply = lib.mapAttrs (published field);
      inherit description;
    };
in
{
  options = {
    base = mkOption {
      type = types.deferredModule;
      default = { };
      apply = published "base" "base";
      description = "NixOS config of every host, assembled by the files under `modules/features/base/<tool>`.";
    };

    profiles = nixosNamespace "profiles" "Roles a host names in `profile`, one folder each under `modules/features/profiles/<name>`.";

    features = nixosNamespace "features" "NixOS modules a host names in `features`, one folder each under `modules/features/<name>`.";

    hardware = nixosNamespace "hardware" "Hardware modules a host names in `hardware`, one folder each under `modules/features/hardware/<name>`.";

    apps = mkOption {
      type = types.attrsOf (
        types.submodule {
          options = {
            nixos = mkOption {
              type = types.deferredModule;
              default = { };
              description = "NixOS part of the app, imported once by every host that names it.";
            };

            homeManager = mkOption {
              type = types.deferredModule;
              default = { };
              description = "home-manager part of the app, imported once for every user who gets it.";
            };
          };
        }
      );
      default = { };
      description = "Apps a host or user names in `apps`, with their config, one folder each under `modules/apps/<name>`.";
    };
  };
}
