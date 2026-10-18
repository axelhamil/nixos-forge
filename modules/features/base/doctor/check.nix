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
        specialisation.signed.configuration = {
          imports = [ features.auto-upgrade ];
          update.signers = [ (import "${pkgs.path}/nixos/tests/ssh-keys.nix" pkgs).snakeOilEd25519PublicKey ];
        };
      };
    testScripts = [ ./test.py ];
  };
}
