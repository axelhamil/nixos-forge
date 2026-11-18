{ lib, config, ... }:

let
  inherit (config.helpers) testMachineScript;

  diskoLauncher = ''machine = create_machine(start_command=" ".join(start_command), **kwargs)'';
  launcherWithoutNetworkCard = "machine = create_machine(start_command=machine_command(*start_command), **kwargs)";

  withoutNetworkCard =
    script:
    lib.throwIfNot (lib.hasInfix diskoLauncher script)
      "helpers.diskTest: disko no longer starts its booted machine with `${diskoLauncher}`, so it may get QEMU's default network card; update the launcher in modules/flake/helpers/test-machine.nix"
      (builtins.replaceStrings [ diskoLauncher ] [ launcherWithoutNetworkCard ] script);

  diskTestModule =
    seconds: test:
    { config, ... }:
    {
      globalTimeout = seconds;
      testScript = lib.mkForce (
        { nodes, ... }:
        testMachineScript config.hostPkgs + withoutNetworkCard (test.config.testScript { inherit nodes; })
      );
    };
in
{
  helpers.qemuBinary =
    pkgs:
    (import (pkgs.path + "/nixos/lib/qemu-common.nix") { inherit (pkgs) lib stdenv; }).qemuBinary
      pkgs.qemu_test;

  helpers.testMachineScript = pkgs: ''
    qemu = "${config.helpers.qemuBinary pkgs}"
    firmware = "${pkgs.OVMF.firmware}"
    firmware_vars = "${pkgs.OVMF.variables}"
    ${builtins.readFile ./test-machine/test-machine.py}
  '';

  helpers.secureMachineScript =
    pkgs:
    let
      firmware = pkgs.OVMFFull;

      prepareSecureMachine = pkgs.writeShellApplication {
        name = "prepare-secure-machine";
        runtimeInputs = [ pkgs.swtpm ];
        runtimeEnv.FIRMWARE_VARS = firmware.variables;
        text = builtins.readFile ./test-machine/prepare-secure-machine.sh;
      };

      secureMachine = lib.concatStringsSep " " [
        (config.helpers.qemuBinary pkgs)
        "-m 1024"
        "-machine q35,smm=on"
        "-global driver=cfi.pflash01,property=secure,value=on"
        "-drive if=pflash,format=raw,unit=0,readonly=on,file=${firmware.firmware}"
        "-tpmdev emulator,id=tpm0,chardev=chrtpm"
        "-device tpm-tis,tpmdev=tpm0"
        "-netdev user,id=tpm-wakeup"
      ];
    in
    ''
      secure_machine = "${secureMachine}"
      prepare_secure_machine = "${lib.getExe prepareSecureMachine}"
      ${builtins.readFile ./test-machine/secure-machine.py}
    '';

  helpers.storeOnDiskTest =
    { lib, pkgs, ... }:
    {
      disko.tests.extraConfig = {
        fileSystems."/nix/store" = lib.mkOverride 40 {
          device = "nix-store";
          fsType = "9p";
          neededForBoot = true;
          options = [
            "trans=virtio"
            "version=9p2000.L"
            "cache=loose"
            "nofail"
          ];
        };

        boot.initrd.systemd.services.initrd-find-nixos-closure = {
          wants = [ "sysroot-nix-store.mount" ];
          after = [ "sysroot-nix-store.mount" ];
          unitConfig.RequiresMountsFor = lib.mkForce "/sysroot/nix";
        };
      };

      disko.tests.extraChecks = lib.mkBefore ''
        cryptsetup = "${lib.getExe' pkgs.cryptsetup "cryptsetup"}"
        ${builtins.readFile ./test-machine/store-on-disk.py}
      '';
    };

  helpers.diskTest = seconds: test: test.extend { modules = [ (diskTestModule seconds test) ]; };
}
