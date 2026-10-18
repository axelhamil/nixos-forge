{ config, ... }:

{
  perSystem = config.helpers.appCheck "neovim" {
    testScripts = [
      ../../flake/helpers/as-tester.py
      ./test.py
    ];
  };
}
