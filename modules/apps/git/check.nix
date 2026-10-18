{ config, ... }:

{
  perSystem = config.helpers.appCheck "git" {
    testScripts = [
      ../../flake/helpers/as-tester.py
      ./test.py
    ];
  };
}
