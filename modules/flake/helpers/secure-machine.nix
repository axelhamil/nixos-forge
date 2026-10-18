{
  helpers.secureMachineScript =
    pkgs:
    let
      inherit (pkgs) lib;

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
        "-drive if=pflash,format=raw,unit=0,readonly=on,file=${firmware.firmware}"
        "-tpmdev emulator,id=tpm0,chardev=chrtpm"
        "-device tpm-tis,tpmdev=tpm0"
      ];
    in
    ''
      secure_machine = "${secureMachine}"
      prepare_secure_machine = "${lib.getExe prepareSecureMachine}"
    '';
}
