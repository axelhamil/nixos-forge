{
  flake.modules.nixos.desktop =
    { pkgs, ... }:
    {
      networking.networkmanager.enable = true;
      boot.kernelPackages = pkgs.linuxPackages_latest;

      environment.persistence."/persist".directories = [
        "/etc/NetworkManager/system-connections"
        "/var/lib/NetworkManager"
      ];
    };
}
