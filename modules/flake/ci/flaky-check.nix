{ lib, ... }:

{
  perSystem =
    {
      pkgs,
      self',
      system,
      ...
    }:
    let
      fakeNix = pkgs.writeShellApplication {
        name = "nix";
        runtimeInputs = [ pkgs.coreutils ];
        text = builtins.readFile ./flaky-fake-nix.sh;
      };

      ciFlaky = import ./_ci-flaky.nix {
        inherit pkgs;
        nix = fakeNix;
      };

      fakeEvalJobs = pkgs.writeShellApplication {
        name = "nix-eval-jobs";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          echo "nix-eval-jobs $*" >>"$STATE/calls"
          cat "$EVALUATED_FILE"
        '';
      };

      flakyHistory = import ./_ci-flaky-history.nix {
        inherit pkgs;
        inherit (self'.packages) forge-ci;
        nix = fakeNix;
        nix-eval-jobs = fakeEvalJobs;
      };

      test = pkgs.writeShellApplication {
        name = "flaky-test";
        runtimeInputs = [
          pkgs.jq
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.gnused
          pkgs.git
        ];
        runtimeEnv = {
          CI_FLAKY = lib.getExe ciFlaky;
          CI_FLAKY_PLAN = lib.getExe flakyHistory.plan;
          CI_FLAKY_REPORT = lib.getExe flakyHistory.report;
          CI_BADGES = lib.getExe self'.packages.ci-badges;
        };
        excludeShellChecks = [ "SC2016" ];
        text = builtins.readFile ./flaky-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        ci-flaky = pkgs.runCommand "ci-flaky" { } ''
          cd "$TMPDIR"
          export HOME=$TMPDIR
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
