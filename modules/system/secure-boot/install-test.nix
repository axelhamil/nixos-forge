{
  flake.modules.nixos.secure-boot =
    {
      lib,
      pkgs,
      options,
      ...
    }:
    let
      qemu = import "${pkgs.path}/nixos/lib/qemu-common.nix" {
        inherit lib;
        inherit (pkgs) stdenv;
      };
      firmware = pkgs.OVMFFull;

      prepareSecureMachine = pkgs.writeShellApplication {
        name = "prepare-secure-machine";
        runtimeInputs = [ pkgs.swtpm ];
        runtimeEnv.FIRMWARE_VARS = firmware.variables;
        text = builtins.readFile ./prepare-secure-machine.sh;
      };

      secureMachine = lib.concatStringsSep " " [
        (qemu.qemuBinary pkgs.qemu_test)
        "-m 1024"
        "-machine q35,smm=on"
        "-global driver=cfi.pflash01,property=secure,value=on"
        "-virtfs local,path=/nix/store,security_model=none,mount_tag=nix-store"
        "-drive if=pflash,format=raw,unit=0,readonly=on,file=${firmware.firmware}"
        "-tpmdev emulator,id=tpm0,chardev=chrtpm"
        "-device tpm-tis,tpmdev=tpm0"
      ];
    in
    {
      config = lib.optionalAttrs (options ? disko) {
        disko.tests.extraChecks = lib.mkIf pkgs.stdenv.hostPlatform.isx86_64 (
          lib.mkAfter ''
            secure_machine = "${secureMachine}"
            prepare_secure_machine = "${lib.getExe prepareSecureMachine}"
            ${builtins.readFile ./secure-boot-test.py}
          ''
        );
      };
    };
}
