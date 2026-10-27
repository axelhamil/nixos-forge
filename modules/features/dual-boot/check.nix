{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers) testRevision secureMachineScript diskTest;
  inherit (config) base features;

  dualBootTest =
    case: secure:
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      fakeWindows = pkgs.writeShellApplication {
        name = "fake-windows";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.dosfstools
          pkgs.gnused
          pkgs.gptfdisk
          pkgs.ntfs3g
          pkgs.parted
          pkgs.util-linux
        ];
        runtimeEnv = {
          WINDOWS_EFI = pkgs.edk2-uefi-shell.efi;
          STARTUP_SCRIPT = ./startup.nsh;
        };
        text = builtins.readFile ./fake-windows.sh;
      };
    in
    {
      dualBoot.windowsPartition = "/dev/disk/by-label/Windows";

      disko.tests.extraChecks = lib.mkMerge [
        (lib.mkOrder 1350 ''
          fake_windows = "${lib.getExe fakeWindows}"
          sgdisk = "${lib.getExe' pkgs.gptfdisk "sgdisk"}"
          cryptsetup = "${lib.getExe' pkgs.cryptsetup "cryptsetup"}"
          qemu_img = "${lib.getExe' pkgs.qemu-utils "qemu-img"}"
          partprobe = "${lib.getExe' pkgs.parted "partprobe"}"
          mkntfs = "${lib.getExe' pkgs.ntfs3g "mkntfs"}"
          install_beside_windows = "${lib.getExe config.system.build.installBesideWindows} dual-boot install-beside-windows"
          tpm_unlocks = ${if secure then "True" else "False"}
          expected_secure_boot = "${if secure then "01" else "00"}"
          ${lib.optionalString (!secure) (secureMachineScript pkgs)}
          ${builtins.readFile ./test-lib.py}
        '')
        (lib.mkIf (case == "same-disk") (lib.mkOrder 1400 (builtins.readFile ./same-disk-test.py)))
        (lib.mkIf (!secure) (
          lib.mkOrder 1650 ''
            state = machine.state_dir
            installer = next(m for m in driver.machines_qemu if m.name == "machine")
          ''
        ))
        (lib.mkOrder 1700 (
          {
            separate-disk = builtins.readFile ./separate-disk-test.py;
            same-disk = builtins.readFile ./second-windows-test.py;
          }
          .${case}
          + builtins.readFile ./dual-boot-test.py
        ))
      ];
    };

  dualBootMachine =
    system: case: secure:
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        base
        features.disk
        features.dual-boot
        (dualBootTest case secure)
        testRevision
        {
          nixpkgs.hostPlatform = system;
          networking.hostName = "dual-boot";
          system.stateVersion = "26.05";
        }
      ]
      ++ lib.optional secure features.secure-boot;
    };
  failingHook =
    system: pkgs:
    let
      machine = (dualBootMachine system "separate-disk" true).extendModules {
        modules = [
          {
            boot.loader.external.installHook = lib.mkForce (pkgs.writeShellScript "failing-hook" "exit 3");
          }
        ];
      };
    in
    pkgs.runCommand "feature-dual-boot-hook-failure" { } ''
      if ${machine.config.system.build.installBootLoader}; then
        echo "the boot loader install succeeded although lanzaboote failed" >&2
        exit 1
      fi
      touch $out
    '';
in
{
  perSystem =
    { system, pkgs, ... }:
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        feature-dual-boot =
          diskTest 2500
            (dualBootMachine system "separate-disk" true).config.system.build.installTest;
        feature-dual-boot-same-disk =
          diskTest 2200
            (dualBootMachine system "same-disk" true).config.system.build.installTest;
        feature-dual-boot-systemd-boot =
          diskTest 1500
            (dualBootMachine system "separate-disk" false).config.system.build.installTest;
        feature-dual-boot-hook-failure = failingHook system pkgs;
      };
    };
}
