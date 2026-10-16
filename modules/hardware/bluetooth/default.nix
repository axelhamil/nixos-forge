{
  flake.modules.nixos.bluetooth = {
    hardware.bluetooth = {
      enable = true;
      powerOnBoot = true;
      settings.General.Experimental = true;
    };

    environment.persistence."/persist".directories = [ "/var/lib/bluetooth" ];
  };
}
