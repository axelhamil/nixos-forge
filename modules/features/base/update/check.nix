{ config, ... }:

let
  inherit (config) base;
in
{
  perSystem = config.helpers.vmCheck "base-update" {
    nodes.desktop =
      { pkgs, ... }:
      {
        imports = [ base ];
        update.flake = "/srv/nixos";
        environment.systemPackages = [ pkgs.git ];
      };
    testScripts = [ ./test.py ];
  };
}
