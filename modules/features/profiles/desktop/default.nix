{
  profiles.desktop =
    { lib, pkgs, ... }:
    {
      networking.networkmanager.enable = true;
      boot.kernelPackages = lib.mkDefault pkgs.linuxPackages_latest;

      services.scx = {
        enable = true;
        scheduler = "scx_lavd";
      };

      services.power-profiles-daemon.enable = true;

      systemd.oomd = {
        enableRootSlice = true;
        enableUserSlices = true;
      };

      systemd.services.nix-daemon.serviceConfig.ManagedOOMPreference = "avoid";

      boot.tmp.useTmpfs = true;

      zramSwap.enable = true;

      environment.persistence."/persist".directories = [
        "/etc/NetworkManager/system-connections"
        "/var/lib/NetworkManager"
      ];

      doctor.ephemeralPaths = [
        "/etc/wpa_supplicant/imperative.conf"
        "/var/lib/power-profiles-daemon"
      ];
    };
}
