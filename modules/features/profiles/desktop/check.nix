{ config, ... }:

{
  perSystem = config.helpers.profileCheck "desktop" {
    machine.environment.persistence."/persist".enable = true;
    testScripts = [
      ./test.py
      ./persistence-test.py
    ];
  };
}
