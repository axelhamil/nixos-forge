{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.starship.homeManager.programs.starship = mkDefaults {
    enable = true;
    presets = [ "plain-text-symbols" ];
  };
}
