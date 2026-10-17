{ config, ... }:

let
  inherit (config) features;
in
{
  perSystem = config.helpers.baseCheck "doctor" {
    machine =
      { pkgs, ... }:
      {
        update.flake = "/srv/nixos";
        environment.systemPackages = [
          pkgs.git
          pkgs.jq
        ];
        users.users.alice.isNormalUser = true;
        specialisation.upgrades.configuration.imports = [ features.auto-upgrade ];
      };
    testScripts = [ ./test.py ];
  };
}
