{ config, ... }:

{
  perSystem = config.helpers.baseCheck "nix" {
    machine.specialisation.forked.configuration = {
      nix.optimise.automatic = false;
      programs.nh.clean.extraArgs = "--keep 3";
    };
    testScripts = [ ./test.py ];
  };
}
