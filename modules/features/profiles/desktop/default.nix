{ config, ... }:

let
  inherit (config) hardware;
in
{
  profiles.desktop =
    { lib, pkgs, ... }:
    {
      imports = [
        hardware.firmware
        hardware.audio
        hardware.bluetooth
      ];

      networking.networkmanager.enable = true;
      boot.kernelPackages = lib.mkDefault pkgs.linuxPackages_latest;

      services.scx = {
        enable = true;
        scheduler = "scx_lavd";
      };

      services.power-profiles-daemon.enable = true;

      services.printing.enable = true;

      services.avahi = {
        enable = true;
        nssmdns4 = true;
      };

      systemd.oomd = {
        enableRootSlice = true;
        enableUserSlices = true;
      };

      systemd.services.nix-daemon.serviceConfig.ManagedOOMPreference = "avoid";

      boot.tmp.useTmpfs = true;

      environment.persistence."/persist".directories = [
        "/etc/NetworkManager/system-connections"
        "/var/lib/NetworkManager"
      ];

      doctor.ephemeralPaths = [
        "/etc/wpa_supplicant/imperative.conf"
        "/etc/printcap"
        "/var/lib/cups"
        "/var/cache/cups"
        "/var/lib/power-profiles-daemon"
      ];
    };
}
