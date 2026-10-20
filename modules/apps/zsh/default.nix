{ lib, config, ... }:

let
  inherit (config.helpers) mkDefaults relativeToHome;
in
{
  apps.zsh.nixos.programs.zsh = {
    enable = true;
    enableGlobalCompInit = lib.mkDefault false;
  };

  apps.zsh.homeManager =
    { config, ... }:
    {
      programs.zsh = mkDefaults {
        enable = true;
        dotDir = "${config.xdg.configHome}/zsh";
        autosuggestion.enable = true;
        syntaxHighlighting.enable = true;
        history = {
          size = 100000;
          save = 100000;
          ignoreAllDups = true;
          share = true;
          path = "${config.xdg.stateHome}/zsh/history";
        };
      };

      home.persistence."/persist".directories = [ (relativeToHome config "${config.xdg.stateHome}/zsh") ];
    };
}
