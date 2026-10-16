{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.homeManager.starship.programs.starship = mkDefaults {
    enable = true;
    presets = [ "plain-text-symbols" ];
  };
}
