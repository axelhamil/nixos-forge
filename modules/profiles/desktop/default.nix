{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  flake.modules.nixos.desktop =
    { lib, pkgs, ... }:
    {
      key = "nixos-starter/profiles/desktop";

      imports = with nixos; [
        firmware
        audio
        bluetooth
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
        "/var/lib/power-profiles-daemon"
        "/var/lib/cups"
      ];
    };
}
