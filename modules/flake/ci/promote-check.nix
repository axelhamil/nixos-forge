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
        name = "promote-test";
        runtimeInputs = [
          pkgs.git
          pkgs.jq
          pkgs.coreutils
          pkgs.gnugrep
        ];
        runtimeEnv = {
          RUNS = ./_fixtures/runs;
          CI_PROMOTE = lib.getExe self'.packages.ci-promote;
        };
        text = builtins.readFile ./promote-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        ci-promote = pkgs.runCommand "ci-promote" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
