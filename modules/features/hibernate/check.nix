{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers)
    hostsOn
    hasDisk
    testRevision
    storeOnDiskTest
    ;
  inherit (config) base features;

  hostConfig = name: config.flake.nixosConfigurations.${name};

  testSwap.hibernate.swapSize = lib.mkForce "1G";

  hibernatesOnDisk = name: hasDisk name && (hostConfig name).options ? hibernate;

  resumeTest =
    { lib, pkgs, ... }:
    let
      qemu = import "${pkgs.path}/nixos/lib/qemu-common.nix" {
        inherit lib;
        inherit (pkgs) stdenv;
      };
    in
    {
      imports = [ storeOnDiskTest ];

      disko.tests.extraConfig = {
        powerManagement.powerDownCommands = "systemctl --no-block stop backdoor.service";
        powerManagement.resumeCommands = "systemctl --no-block restart backdoor.service";
      };

      disko.tests.extraChecks = lib.mkAfter ''
        qemu = "${qemu.qemuBinary pkgs.qemu_test}"
        firmware = "${pkgs.OVMF.firmware}"
        firmware_vars = "${pkgs.OVMF.variables}"
        ${builtins.readFile ./resume-test.py}
      '';
    };

  hibernatingMachine =
    system:
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        base
        features.disk
        features.hibernate
        testSwap
        resumeTest
        testRevision
        {
          nixpkgs.hostPlatform = system;
          networking.hostName = "hibernate";
          zramSwap.enable = true;
          system.stateVersion = "26.05";
        }
      ];
    };
in
{
  perSystem =
    { system, ... }:
    {
      checks =
        lib.concatMapAttrs (name: _: {
          "${name}-disk" =
            lib.mkForce
              ((hostConfig name).extendModules {
                modules = [
                  testSwap
                  testRevision
                ];
              }).config.system.build.installTest;
        }) (lib.filterAttrs (name: _: hibernatesOnDisk name) (hostsOn system))
        // lib.optionalAttrs (system == "x86_64-linux") {
          feature-hibernate = (hibernatingMachine system).config.system.build.installTest;
        };
    };
}
