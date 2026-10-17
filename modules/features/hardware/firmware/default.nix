{
  hardware.firmware = {
    hardware.enableRedistributableFirmware = true;

    services.fwupd.enable = true;

    environment.persistence."/persist".directories = [ "/var/lib/fwupd" ];
  };
}
