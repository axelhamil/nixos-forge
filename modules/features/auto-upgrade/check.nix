{ config, ... }:

let
  inherit (config) base features;
  inherit (config.helpers) vmCheck;

  stub =
    pkgs:
    pkgs.writeShellScript "nixos-rebuild-stub" ''
      if [ -e /run/rebuild-fails ]; then
        echo "error: attribute 'machine' missing" >&2
        exit 1
      fi
      echo "$*" >> /var/log/nixos-rebuild-calls
      if [ -e /run/cache-lacks-version ]; then
        shift 3
        exec nix-build --no-out-link --option substituters "" "$@" -E \
          'derivation { name = "not-in-cache"; system = builtins.currentSystem; builder = "/bin/sh"; }'
      fi
      ln -sfn "$(readlink -f /run/current-system)" /nix/var/nix/profiles/system
    '';
in
{
  perSystem.imports = [
    (vmCheck "feature-auto-upgrade" {
      nodes = {
        machine =
          { pkgs, ... }:
          {
            imports = [
              base
              features.auto-upgrade
            ];
            update.flake = "github:alice/nixos";
            update.mirrors = [ "file:///srv/mirror.git" ];
            autoUpgrade.allowLocalBuild = false;
            autoUpgrade.rebuildCommand = toString (stub pkgs);
            environment.systemPackages = [ pkgs.git ];
            specialisation.variant.configuration = {
              autoUpgrade.operation = "switch";
              environment.etc.specialisation.text = "variant";
            };
            specialisation.pinned.configuration.environment.etc.specialisation.text = "pinned";
          };

        desktop =
          { pkgs, ... }:
          {
            imports = [
              base
              features.auto-upgrade
            ];
            update.flake = "/home/bob/nixos";
            update.mirrors = [ "file:///srv/mirror.git" ];
            autoUpgrade.rebuildCommand = toString (stub pkgs);
            environment.systemPackages = [ pkgs.git ];
            users.users.bob.isNormalUser = true;
            users.users.alice = {
              isNormalUser = true;
              extraGroups = [ "wheel" ];
            };
          };
      };
      testScripts = [ ./test.py ];
    })

    (vmCheck "feature-auto-upgrade-signed" {
      nodes.machine =
        { pkgs, ... }:
        let
          keys = import "${pkgs.path}/nixos/tests/ssh-keys.nix" pkgs;
        in
        {
          imports = [
            base
            features.auto-upgrade
          ];
          update.flake = "github:alice/nixos";
          update.mirrors = [
            "file:///srv/primary.git"
            "file:///srv/mirror.git"
          ];
          update.signers = [ keys.snakeOilEd25519PublicKey ];
          autoUpgrade.allowLocalBuild = false;
          autoUpgrade.rebuildCommand = toString (stub pkgs);
          environment.systemPackages = [
            pkgs.git
            pkgs.gnupg
            pkgs.openssh
          ];
          environment.etc.deploy-key.source = keys.snakeOilEd25519PrivateKey;
          systemd.services.auto-upgrade.path = [ pkgs.gnupg ];
        };
      testScripts = [ ./signed-test.py ];
    })
  ];
}
