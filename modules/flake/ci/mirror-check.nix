{ lib, config, ... }:

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
        name = "mirror-test";
        runtimeInputs = [
          pkgs.git
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.python3
        ];
        runtimeEnv.CI_MIRROR = lib.getExe self'.packages.ci-mirror;
        text = builtins.readFile ./mirror-test.sh;
      };
    in
    {
      checks = scriptCheck pkgs "ci-mirror" test;
    };
}
