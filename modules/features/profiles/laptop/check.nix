{ config, ... }:

{
  perSystem = config.helpers.profileCheck "laptop" {
    testScripts = [
      ../desktop/test.py
      ./test.py
    ];
  };
}
