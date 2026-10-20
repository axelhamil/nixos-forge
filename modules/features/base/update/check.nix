{ config, ... }:

let
  inherit (config) base;

  flake =
    pkgs:
    let
      activate =
        variant:
        pkgs.writeShellScript "switch-to-configuration" ''
          echo "${variant} $1" >> /var/log/update-activations
        '';

      build = pkgs.writeText "build-update-test-system" ''
        set -e
        ${pkgs.coreutils}/bin/mkdir -p $out/bin $out/specialisation/gaming/bin
        ${pkgs.coreutils}/bin/ln -s ${activate "base"} $out/bin/switch-to-configuration
        ${pkgs.coreutils}/bin/ln -s ${activate "gaming"} $out/specialisation/gaming/bin/switch-to-configuration
      '';
    in
    pkgs.writeText "flake.nix" ''
      {
        outputs = _: {
          nixosConfigurations.desktop.config.system.build.toplevel = derivation {
            name = "update-test-system";
            system = "${pkgs.stdenv.hostPlatform.system}";
            builder = "${pkgs.bash}/bin/bash";
            args = [ "${build}" ];
          };
        };
      }
    '';
in
{
  perSystem = config.helpers.vmCheck "base-update" {
    nodes.desktop =
      { lib, pkgs, ... }:
      {
        imports = [ base ];
        update.flake = "/srv/nixos";
        environment.systemPackages = [ pkgs.git ];
        environment.etc."update-test/flake.nix".source = flake pkgs;
        nix.settings.sandbox = false;
        nix.settings.substituters = lib.mkForce [ ];
      };
    testScripts = [ ./test.py ];
  };
}
