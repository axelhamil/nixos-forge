{ config, ... }:

let
  inherit (config.helpers) appPart;
in
{
  perSystem = config.helpers.appCheck "delta" {
    machine.home-manager.users.tester.imports = [ (appPart "git" "homeManager") ];
    testScripts = [
      ../../flake/helpers/as-tester.py
      ./test.py
    ];
  };
}
