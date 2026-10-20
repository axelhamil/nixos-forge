{ config, ... }:

{
  perSystem = config.helpers.appCheck "gh" {
    machine.impermanence.wipeHome = true;
    testScripts = [ ./test.py ];
  };
}
