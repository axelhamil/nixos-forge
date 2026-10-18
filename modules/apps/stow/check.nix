{ config, ... }:

let
  inherit (config.helpers) appPart;
in
{
  perSystem = config.helpers.appCheck "stow" {
    machine =
      { lib, pkgs, ... }:
      let
        users = [
          "tester"
          "mirror"
        ];
      in
      {
        imports = [ (appPart "zsh" "nixos") ];
        users.users = lib.genAttrs users (_: {
          isNormalUser = true;
          shell = pkgs.zsh;
        });
        home-manager.users = lib.genAttrs users (_: {
          imports = map (app: appPart app "homeManager") [
            "zsh"
            "git"
            "neovim"
            "stow"
          ];
          home.stateVersion = "26.05";
        });
        specialisation.fewer.configuration.home-manager.users.tester.stow.packages = [ "zsh" ];
      };
    testScripts = [ ./test.py ];
  };
}
