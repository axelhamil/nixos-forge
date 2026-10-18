{ config, ... }:

{
  perSystem = config.helpers.featureCheck "printing" {
    machine.environment.persistence."/persist".enable = true;
    testScripts = [ ./test.py ];
  };
}
