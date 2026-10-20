{
  hardware.firmware =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    {
      hardware.enableRedistributableFirmware = true;
      hardware.cpu.intel.updateMicrocode = lib.mkDefault (
        config.hardware.enableRedistributableFirmware && pkgs.stdenv.hostPlatform.isx86
      );
      hardware.cpu.amd.updateMicrocode = lib.mkDefault (
        config.hardware.enableRedistributableFirmware && pkgs.stdenv.hostPlatform.isx86
      );

      services.fwupd.enable = true;

      doctor.ephemeralPaths = [ "/var/lib/fwupd" ];
    };
}
