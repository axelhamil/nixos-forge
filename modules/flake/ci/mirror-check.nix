{ lib, ... }:

{
  perSystem =
    {
      pkgs,
      system,
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
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        ci-mirror = pkgs.runCommand "ci-mirror" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
