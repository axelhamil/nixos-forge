{
  lib,
  inputs,
  config,
  options,
  ...
}:

let
  hostType = options.configurations.nixos.type.nestedTypes.elemType;
  person = lib.head (lib.attrNames config.users);

  registryHost =
    gaming:
    lib.evalModules {
      modules = hostType.getSubModules ++ [
        {
          _module.args.name = "variants";
          stateVersion = "26.05";
          users.${person} = { };
          specialisations = { inherit gaming; };
        }
      ];
    };

  host = registryHost {
    features = [ "nix-ld" ];
    hardware = [ "bluetooth" ];
    apps = [
      "gamemode"
      "mangohud"
      "hello"
    ];
    module = {
      environment.variables.VARIANT_MODULE = "applied";
      boot.kernelModules = [ "ntsync" ];
      home-manager.sharedModules = [ { home.file.variant.text = "gaming"; } ];
    };
  };

  steamVariant =
    (inputs.nixpkgs.lib.nixosSystem {
      modules = [
        (registryHost { apps = [ "steam" ]; }).config.module
        {
          nixpkgs.hostPlatform = "x86_64-linux";
          fileSystems."/" = {
            device = "/dev/null";
            fsType = "ext4";
          };
          boot.loader.grub.enable = false;
        }
      ];
    }).config.specialisation.gaming.configuration;

  steamKept = lib.any (
    entry: entry.directory == ".steam"
  ) steamVariant.home-manager.users.${person}.home.persistence."/persist".directories;
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks.specialisations =
        assert lib.assertMsg steamKept
          "the homeManager part of an app named by a variant did not reach its users";
        assert lib.assertMsg (lib.elem "ntsync" steamVariant.boot.kernelModules)
          "steam does not load ntsync, so /dev/ntsync is missing for Proton";
        pkgs.testers.runNixOSTest {
          name = "specialisations";
          node.pkgsReadOnly = false;
          nodes.machine = {
            imports = [
              host.config.module
              config.helpers.testRevision
            ];
            virtualisation.memorySize = 2048;
          };
          testScript = builtins.readFile ./specialisations-test.py;
        };
    };
}
