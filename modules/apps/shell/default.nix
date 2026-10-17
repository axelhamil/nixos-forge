{ config, ... }:

let
  inherit (config.helpers) appPart;

  tools = [
    "zsh"
    "starship"
    "git"
    "neovim"
    "cli"
  ];
in
{
  apps.shell = {
    nixos.imports = [ (appPart "zsh" "nixos") ];

    homeManager = {
      xdg.enable = true;

      imports = map (tool: appPart tool "homeManager") tools;
    };
  };
}
