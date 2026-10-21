{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers) testRevision baseFlake diskTest;

  host = "example";

  installedAsInTheTest =
    next:
    { lib, modulesPath, ... }:
    {
      imports = [
        "${modulesPath}/testing/test-instrumentation.nix"
        "${modulesPath}/profiles/qemu-guest.nix"
        next.config.disko.tests.extraConfig
      ];

      fileSystems."/nix/store" = lib.mkForce {
        device = "nix-store";
        fsType = "9p";
        neededForBoot = true;
        options = [
          "trans=virtio"
          "version=9p2000.L"
          "cache=loose"
        ];
      };
    };

  migrationTest =
    next: base:
    { lib, ... }:
    let
      nextSystem =
        (next.extendModules {
          modules = [
            testRevision
            (installedAsInTheTest next)
          ];
        }).config;
    in
    {
      imports = [ testRevision ];

      environment.etc."migration-test/next".source = nextSystem.system.build.toplevel;

      disko.tests.extraChecks = lib.mkForce ''
        user = ${builtins.toJSON (lib.head (lib.attrNames config.configurations.nixos.${host}.users))}
        failing_on_shared_store = ${
          builtins.toJSON (
            map (name: "${name}.service") (
              lib.filter (lib.hasPrefix "home-manager-") (lib.attrNames nextSystem.systemd.services)
            )
            ++ [ "systemd-boot-check-no-failures.service" ]
          )
        }
        wipes_kept_home = ${
          if nextSystem.impermanence.wipeHome && !(base.config.impermanence.wipeHome or false) then
            "True"
          else
            "False"
        }
        unlock_prompt = ${builtins.toJSON nextSystem.disk.unlockPrompt}
        ${builtins.readFile ./migration-test.py}
      '';
    };

  migrationChecks =
    next: base:
    lib.optionalAttrs (next.config ? disko && base.config ? disko) {
      feature-migration =
        diskTest 1500
          (base.extendModules { modules = [ (migrationTest next base) ]; }).config.system.build.installTest
        // {
          baseInput = "migration-base";
        };
    };

  machine =
    modules:
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        config.base
        {
          nixpkgs.hostPlatform = "x86_64-linux";
          system.stateVersion = "26.05";
        }
      ]
      ++ modules;
    };

  withoutDisk = machine [ ];
  withDisk = machine [ config.features.disk ];
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks =
        lib.optionalAttrs (
          config.flake.nixosConfigurations ? ${host} && baseFlake.nixosConfigurations ? ${host}
        ) (migrationChecks config.flake.nixosConfigurations.${host} baseFlake.nixosConfigurations.${host})
        // {
          feature-migration-without-disk =
            let
              generated = lib.mapAttrs (_: check: check.drvPath) (
                migrationChecks withoutDisk withDisk // migrationChecks withDisk withoutDisk
              );
            in
            assert generated == { };
            pkgs.writeText "feature-migration-without-disk" "";
        };
    };
}
