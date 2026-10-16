{ config, ... }:

let
  inherit (config.helpers) mkDefaults relativeToHome;
in
{
  flake.modules.homeManager.neovim =
    { config, pkgs, ... }:
    {
      programs.neovim =
        mkDefaults {
          enable = true;
          defaultEditor = true;
          viAlias = true;
          vimAlias = true;
          sideloadInitLua = true;
        }
        // {
          extraPackages = with pkgs; [
            gcc
            tree-sitter
            ripgrep
            fd
          ];
        };

      home.persistence."/persist".directories = map (relativeToHome config) [
        "${config.xdg.dataHome}/nvim"
        "${config.xdg.stateHome}/nvim"
      ];
    };
}
