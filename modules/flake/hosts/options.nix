{ lib, config, ... }:

let
  inherit (lib) mkOption types;

  hostOptions = types.submodule {
    options = {
      system = mkOption {
        type = types.enum config.systems;
        default = "x86_64-linux";
        description = "Platform the host runs on; its checks run on a CI runner of the same platform.";
      };

      module = mkOption {
        type = types.deferredModule;
        description = "NixOS configuration of the host, usually imports of named modules plus host settings.";
      };

      testScript = mkOption {
        type = types.lines;
        default = "";
        description = "Python lines appended to the host boot test, run against the `machine` node.";
      };
    };
  };
in
{
  options.configurations.nixos = mkOption {
    type = types.lazyAttrsOf hostOptions;
    default = { };
    description = "Hosts of the flake. Each one gets a nixosConfigurations entry, a build check and a boot test.";
  };
}
