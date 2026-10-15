{ config, ... }:

{
  perSystem = config.helpers.vmCheck "system-resolved" {
    nodes.machine.imports = [ config.flake.modules.nixos.base ];
    testScripts = [ ./test.py ];
  };
}
