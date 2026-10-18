{ lib, config, ... }:

let
  inherit (config.helpers) installerHostsOn;
in
{
  perSystem =
    { pkgs, system, ... }:
    {
      packages = lib.optionalAttrs (system == "x86_64-linux") {
        try-install = pkgs.writeShellApplication {
          name = "try-install";
          runtimeInputs = [
            pkgs.coreutils
            pkgs.qemu_kvm
            pkgs.swtpm
          ];
          runtimeEnv = {
            INSTALLER_HOSTS = lib.concatStringsSep " " (lib.attrNames (installerHostsOn system));
            FIRMWARE = pkgs.OVMFFull.firmware;
            FIRMWARE_VARS = pkgs.OVMFFull.variables;
          };
          text = builtins.readFile ./try-install.sh;
        };
      };
    };
}
