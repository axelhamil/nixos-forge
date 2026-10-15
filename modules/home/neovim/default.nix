{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.homeManager.neovim =
    { pkgs, ... }:
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
    };
}
