{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers) testRevision storeOnDiskTest;
  inherit (config) base features;

  failingGeneration =
    { lib, pkgs, ... }:
    {
      environment.etc."rollback-test/next-generation".enable = lib.mkForce false;

      systemd.services.broken-upgrade = {
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Type = "oneshot";
          ExecStart = lib.getExe' pkgs.coreutils "false";
        };
      };
    };

  nextGeneration =
    { extendModules, ... }:
    let
      next = (extendModules { modules = [ failingGeneration ]; }).config.system.build.toplevel;
    in
    {
      environment.etc."rollback-test/next-generation".source = next;
    };

  rollbackTest =
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

      disko.tests.extraConfig.imports = [ nextGeneration ];

      disko.tests.extraChecks = lib.mkOrder 1600 (
        (
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
    system: extraModules:
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        base
        features.disk
        features.auto-upgrade
        rollbackTest
        testRevision
        {
          nixpkgs.hostPlatform = system;
          networking.hostName = "rollback";
          update.flake = "github:alice/nixos";
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
        feature-rollback = (upgradingMachine system [ ]).config.system.build.installTest;
        feature-rollback-secure-boot =
          (upgradingMachine system [ features.secure-boot ]).config.system.build.installTest;
      };
    };
}
