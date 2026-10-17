{ config, ... }:

{
  perSystem = config.helpers.appCheck "shell" {
    machine =
      { pkgs, ... }:
      {
        users.users.tester.shell = pkgs.zsh;
      };
    testScripts = [ ./test.py ];
  };
}
