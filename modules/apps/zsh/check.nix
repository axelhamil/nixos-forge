{ config, ... }:

{
  perSystem = config.helpers.appCheck "zsh" {
    machine =
      { pkgs, ... }:
      {
        users.users.tester.shell = pkgs.zsh;
      };
    testScripts = [
      ../../flake/helpers/as-tester.py
      ./test.py
    ];
  };
}
