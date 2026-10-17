{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "system-update" {
    nodes = {
      desktop =
        { pkgs, ... }:
        {
          imports = [ nixos.base ];
          update.flake = "/srv/nixos";
          environment.systemPackages = [ pkgs.git ];
        };

      server = {
        imports = [
          nixos.base
          nixos.auto-upgrade
        ];
        update.flake = "github:alice/nixos";
      };
    };
    testScripts = [ ./test.py ];
  };
}
