{ config, ... }:

{
  perSystem = config.helpers.appCheck "fzf" {
    testScripts = [
      ../../flake/helpers/as-tester.py
      ./test.py
    ];
  };
}
