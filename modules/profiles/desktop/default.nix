{
  flake.modules.nixos.desktop =
    { pkgs, ... }:
    {
      networking.networkmanager.enable = true;
      boot.kernelPackages = pkgs.linuxPackages_latest;
    };
}
