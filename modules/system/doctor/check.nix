{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "system-doctor" {
    nodes.machine =
      { pkgs, ... }:
      {
        imports = [ nixos.base ];
        update.flake = "/srv/nixos";
        environment.systemPackages = [ pkgs.git ];
        users.users.alice.isNormalUser = true;
      };
    testScripts = [ ./test.py ];
  };
}
