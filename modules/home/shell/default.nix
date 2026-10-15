{ config, ... }:

{
  flake.modules.homeManager.shell = {
    xdg.enable = true;

    imports = with config.flake.modules.homeManager; [
      zsh
      starship
      git
      neovim
      cli
    ];
  };
}
