{ config, ... }:

let
  inherit (config) base features;

  stub =
    pkgs:
    pkgs.writeShellScript "nixos-rebuild-stub" ''
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
  perSystem = config.helpers.vmCheck "feature-auto-upgrade" {
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
  };
}
