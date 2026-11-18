{ config, ... }:

let
  inherit (config.helpers) scriptCheck;
in
{
  perSystem =
    {
      pkgs,
      self',
      ...
    }:
    let
      test = pkgs.writeShellApplication {
        name = "try-install-test";
        runtimeInputs = [
          self'.packages.try-install
          pkgs.gnugrep
        ];
        text = builtins.readFile ./try-install-test.sh;
      };
    in
    {
      checks = scriptCheck pkgs "try-install" test;
    };
}
