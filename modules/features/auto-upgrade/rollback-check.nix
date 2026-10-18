{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers) testRevision storeOnDiskTest;
  inherit (config) base features;

  failures = {
    boot-check = {
      unit = "failing-upgrade";
      idle = [ "systemd-boot-check-no-failures.service" ];
      generation.systemd.targets.boot-complete.requires = lib.mkForce [ ];
      service = {
        requiredBy = [ "boot-complete.target" ];
        before = [ "boot-complete.target" ];
      };
    };
    failed-unit = {
      unit = "systemd-boot-check-no-failures";
      idle = [ ];
      generation = { };
      service.wantedBy = [ "multi-user.target" ];
    };
  };

  failingGeneration =
    failure:
    { lib, pkgs, ... }:
    {
      imports = [ failure.generation ];

      environment.etc."rollback-test/next-generation".enable = lib.mkForce false;

      systemd.services.failing-upgrade = failure.service // {
        serviceConfig = {
          Type = "oneshot";
          ExecStart = lib.getExe' pkgs.coreutils "false";
        };
      };
    };

  nextGeneration =
    failure:
    { extendModules, ... }:
    let
      next = (extendModules { modules = [ (failingGeneration failure) ]; }).config.system.build.toplevel;
    in
    {
      environment.etc."rollback-test/next-generation".source = next;
    };

  rollbackTest =
    failure:
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      qemu = import "${pkgs.path}/nixos/lib/qemu-common.nix" {
        inherit lib;
        inherit (pkgs) stdenv;
      };
    in
    {
      imports = [ storeOnDiskTest ];

      disko.tests.extraConfig.imports = [ (nextGeneration failure) ];

      disko.tests.extraChecks = lib.mkOrder 1600 (
        ''
          failed_unit = "${failure.unit}.service"
          idle_units = ${builtins.toJSON failure.idle}
        ''
        + (
          if config.boot ? lanzaboote && config.boot.lanzaboote.enable then
            builtins.readFile ./rollback-secure-machine.py
          else
            ''
              qemu = "${qemu.qemuBinary pkgs.qemu_test}"
              firmware = "${pkgs.OVMF.firmware}"
              firmware_vars = "${pkgs.OVMF.variables}"
              ${builtins.readFile ./rollback-machine.py}
            ''
        )
        + builtins.readFile ./rollback-test.py
      );
    };

  upgradingMachine =
    system: failure: extraModules:
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        base
        features.disk
        features.auto-upgrade
        (rollbackTest failure)
        testRevision
        (
          { pkgs, ... }:
          {
            environment.systemPackages = [ pkgs.git ];
          }
        )
        {
          nixpkgs.hostPlatform = system;
          networking.hostName = "rollback";
          update.flake = "github:alice/nixos";
          update.mirrors = [ "file:///srv/mirror.git" ];
          system.stateVersion = "26.05";
        }
      ]
      ++ extraModules;
    };
in
{
  perSystem =
    { system, ... }:
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        feature-rollback =
          (upgradingMachine system failures.boot-check [ ]).config.system.build.installTest;
        feature-rollback-secure-boot =
          (upgradingMachine system failures.boot-check [ features.secure-boot ])
          .config.system.build.installTest;
        feature-rollback-failed-unit =
          (upgradingMachine system failures.failed-unit [ ]).config.system.build.installTest;
      };
    };
}
