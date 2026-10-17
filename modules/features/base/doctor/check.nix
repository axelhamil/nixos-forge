{ config, ... }:

{
  perSystem = config.helpers.baseCheck "doctor" {
    machine =
      { pkgs, ... }:
      {
        update.flake = "/srv/nixos";
        environment.systemPackages = [ pkgs.git ];
        users.users.alice.isNormalUser = true;
      };
    testScripts = [ ./test.py ];
  };
}
