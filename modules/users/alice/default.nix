{ config, ... }:

let
  inherit (config.flake.modules) nixos homeManager;
in
{
  flake.modules.nixos.alice =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    {
      imports = [ nixos.zsh ];

      users.users.alice = {
        isNormalUser = true;
        extraGroups = [
          "wheel"
          "networkmanager"
        ];
        shell = pkgs.zsh;
        hashedPasswordFile =
          lib.mkIf config.environment.persistence."/persist".enable
            "/persist/passwords/alice";
      };

      home-manager.users.alice.imports = [ homeManager.alice ];

      virtualisation.vmVariant.users.users.alice = {
        hashedPasswordFile = lib.mkForce null;
        initialPassword = "test";
      };
    };

  flake.modules.homeManager.alice = {
    imports = [ homeManager.shell ];

    home.stateVersion = "26.05";
    programs.home-manager.enable = true;
  };
}
