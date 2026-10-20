{
  hardware.bluetooth =
    { lib, ... }:
    {
      hardware.bluetooth = {
        enable = true;
        powerOnBoot = lib.mkDefault true;
        settings.General.Experimental = lib.mkDefault true;
      };

      environment.persistence."/persist".directories = [ "/var/lib/bluetooth" ];
    };
}
