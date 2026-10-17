{
  hardware.firmware =
    { lib, config, ... }:
    {
      hardware.enableRedistributableFirmware = true;
      hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
      hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;

      services.fwupd.enable = true;

      doctor.ephemeralPaths = [ "/var/lib/fwupd" ];
    };
}
