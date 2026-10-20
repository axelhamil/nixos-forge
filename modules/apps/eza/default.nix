{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.eza.homeManager = mkDefaults {
    programs.eza = {
      enable = true;
      git = true;
      enableZshIntegration = false;
      enableBashIntegration = false;
      enableFishIntegration = false;
      enableIonIntegration = false;
      enableNushellIntegration = false;
    };

    home.shellAliases = {
      ll = "eza -l";
      la = "eza -la";
      lt = "eza --tree";
    };
  };
}
