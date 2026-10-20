{ config, ... }:

let
  inherit (config.helpers) relativeToHome;
in
{
  apps.steam.nixos =
    { lib, pkgs, ... }:
    {
      nixpkgs.config.allowUnfreePackages = [
        "steam"
        "steam-unwrapped"
      ];

      programs.steam = {
        enable = true;
        protontricks.enable = lib.mkDefault true;
        extraCompatPackages = [ pkgs.proton-ge-bin ];
      };

      boot.kernelModules = [ "ntsync" ];
    };

  apps.steam.homeManager =
    { config, ... }:
    {
      home.persistence."/persist".directories = [
        ".steam"
        (relativeToHome config "${config.xdg.dataHome}/Steam")
      ];
    };
}
