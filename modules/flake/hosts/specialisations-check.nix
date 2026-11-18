{
  lib,
  config,
  ...
}:

let
  inherit (config.helpers) registryHost registryMachine;
  person = lib.head (lib.attrNames config.users);

  variants =
    gaming:
    registryHost "variants" [
      {
        users.${person} = { };
        specialisations = { inherit gaming; };
      }
    ];

  host = variants {
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
    (registryMachine (variants { apps = [ "steam" ]; }) [
      {
        fileSystems."/" = {
          device = "/dev/null";
          fsType = "ext4";
        };
        boot.loader.grub.enable = false;
      }
    ]).config.specialisation.gaming.configuration;

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
          globalTimeout = 400;
          node.pkgsReadOnly = false;
          nodes.machine = {
            imports = [
              host.module
              config.helpers.testRevision
            ];
            virtualisation.memorySize = 2048;
          };
          testScript = builtins.readFile ./specialisations-test.py;
        };
    };
}
