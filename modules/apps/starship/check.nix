{ config, ... }:

let
  inherit (config.helpers) appPart;
in
{
  perSystem = config.helpers.appCheck "starship" {
    machine =
      { pkgs, ... }:
      {
        imports = [ (appPart "zsh" "nixos") ];
        users.users.tester.shell = pkgs.zsh;
        home-manager.users.tester.imports = [ (appPart "zsh" "homeManager") ];
      };
    testScripts = [
      ../../flake/helpers/as-tester.py
      ./test.py
    ];
  };
}
