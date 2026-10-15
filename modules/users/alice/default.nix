{ config, ... }:

let
  aliceHome = config.flake.modules.homeManager.alice;
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

      programs.zsh.enable = true;

      home-manager.users.alice.imports = [ aliceHome ];

      virtualisation.vmVariant.users.users.alice = {
        hashedPasswordFile = lib.mkForce null;
        initialPassword = "test";
      };
    };

  flake.modules.homeManager.alice = {
    home.stateVersion = "26.05";
    programs.home-manager.enable = true;
  };
}
