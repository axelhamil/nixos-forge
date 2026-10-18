{ config, ... }:

let
  inherit (config.helpers) relativeToHome;
in
{
  apps.zoxide.homeManager =
    { config, lib, ... }:
    {
      programs.zoxide.enable = lib.mkDefault true;

      home.persistence."/persist".directories = [
        (relativeToHome config "${config.xdg.dataHome}/zoxide")
      ];
    };
}
