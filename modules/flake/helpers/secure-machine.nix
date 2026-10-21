{ lib, config, ... }:

{
  helpers.secureMachineScript =
    pkgs:
    let
      firmware = pkgs.OVMFFull;

      prepareSecureMachine = pkgs.writeShellApplication {
        name = "prepare-secure-machine";
        runtimeInputs = [ pkgs.swtpm ];
        runtimeEnv.FIRMWARE_VARS = firmware.variables;
        text = builtins.readFile ./prepare-secure-machine.sh;
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
      ${builtins.readFile ./secure-machine.py}
    '';
}
