{
  hardware.firmware = {
    hardware.enableRedistributableFirmware = true;

    services.fwupd.enable = true;

    doctor.ephemeralPaths = [ "/var/lib/fwupd" ];
  };
}
