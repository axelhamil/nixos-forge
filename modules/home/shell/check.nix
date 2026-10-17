{ config, ... }:

let
  inherit (config) base;
  inherit (config.flake.modules) nixos homeManager;
in
{
  perSystem = config.helpers.vmCheck "home-shell" {
    nodes.machine =
      { pkgs, ... }:
      {
        imports = [
          base
          nixos.zsh
        ];
        users.users.tester = {
          isNormalUser = true;
          shell = pkgs.zsh;
        };
        home-manager.users.tester = {
          imports = [ homeManager.shell ];
          home.stateVersion = "26.05";
        };
      };
    testScripts = [ ./test.py ];
  };
}
