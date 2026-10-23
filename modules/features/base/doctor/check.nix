{ config, ... }:

let
  inherit (config) features;

  forkCheck =
    pkgs: name: check:
    pkgs.writeShellApplication {
      name = "forge-${name}";
      text = ''
        if [ "''${1:-}" = --forge-describe ]; then
          echo '{"contract":1,"name":"${name}","summary":"A check a fork adds","doctor":true}'
          exit 0
        fi
        if [ "''${1:-}" != --forge-doctor ]; then
          exit 2
        fi
        ${check}
      '';
    };
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
        specialisation.forks.configuration.forge.commands = [
          (forkCheck pkgs "backup-age" ''
            echo '{"status":"WARN","check":"Backup age","detail":"last backup 3 days ago","hint":"systemctl start backup"}'
          '')
          (forkCheck pkgs "crash" "exit 1")
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
