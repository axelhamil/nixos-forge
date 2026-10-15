{ config, ... }:

{
  flake.modules.nixos.alice =
    { pkgs, ... }:
    {
      users.users.alice = {
        isNormalUser = true;
        extraGroups = [
          "wheel"
          "networkmanager"
        ];
        shell = pkgs.zsh;
      };

      programs.zsh.enable = true;

      home-manager.users.alice.imports = [ config.flake.modules.homeManager.alice ];

      virtualisation.vmVariant.users.users.alice.initialPassword = "test";
    };

  flake.modules.homeManager.alice = {
    home.stateVersion = "26.05";
    programs.home-manager.enable = true;
  };
}
