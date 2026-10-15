{ config, ... }:

let
  inherit (config.flake.modules) nixos homeManager;
in
{
  perSystem = config.helpers.vmCheck "home-stow" {
    nodes.machine =
      { lib, pkgs, ... }:
      let
        user = {
          isNormalUser = true;
          shell = pkgs.zsh;
        };
        home = {
          imports = [
            homeManager.shell
            homeManager.stow
          ];
          home.stateVersion = "26.05";
        };
      in
      {
        imports = [
          nixos.base
          nixos.zsh
        ];
        users.users = lib.genAttrs [ "tester" "mirror" ] (_: user);
        home-manager.users = lib.genAttrs [ "tester" "mirror" ] (_: home);
        specialisation.fewer.configuration.home-manager.users.tester.stow.packages = [ "zsh" ];
      };
    testScripts = [ ./test.py ];
  };
}
