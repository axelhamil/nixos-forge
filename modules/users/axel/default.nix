{ config, ... }:

let
  homeModules = config.flake.modules.homeManager;
in
{
  flake.modules.nixos.axel = { pkgs, ... }: {
    users.users.axel = {
      isNormalUser = true;
      extraGroups = [
        "wheel"
        "networkmanager"
      ];
      shell = pkgs.zsh;
    };

    programs.zsh.enable = true;

    home-manager.users.axel.imports = [ homeModules.axel ];
  };

  flake.modules.homeManager.axel = {
    home.stateVersion = "26.05";
    programs.home-manager.enable = true;
  };
}
