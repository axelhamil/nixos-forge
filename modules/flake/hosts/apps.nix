{ lib, ... }:

{
  options.apps = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          nixos = lib.mkOption {
            type = lib.types.deferredModule;
            default = { };
            description = "NixOS part of the app, imported once by every host that names it.";
          };

          homeManager = lib.mkOption {
            type = lib.types.deferredModule;
            default = { };
            description = "home-manager part of the app, imported once for every user who gets it.";
          };
        };
      }
    );
    default = { };
    description = "Apps a host or user names in `apps`, with their config, one folder each under `modules/apps/<name>`.";
  };
}
