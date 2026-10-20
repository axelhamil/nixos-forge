{ config, ... }:

let
  inherit (config.helpers) relativeToHome;
in
{
  apps.gh.homeManager =
    { lib, config, ... }:
    {
      programs.gh.enable = lib.mkDefault true;

      home.persistence."/persist".directories = [
        {
          directory = relativeToHome config "${config.xdg.configHome}/gh";
          mode = "0700";
        }
      ];
    };
}
