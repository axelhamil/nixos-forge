{ config, ... }:

let
  inherit (config) base features;
in
{
  perSystem = config.helpers.vmCheck "base-update" {
    nodes = {
      desktop =
        { pkgs, ... }:
        {
          imports = [ base ];
          update.flake = "/srv/nixos";
          environment.systemPackages = [ pkgs.git ];
        };

      server = {
        imports = [
          base
          features.auto-upgrade
        ];
        update.flake = "github:alice/nixos";
      };
    };
    testScripts = [ ./test.py ];
  };
}
