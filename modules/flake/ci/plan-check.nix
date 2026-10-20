{ lib, ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      fakeNix = pkgs.writeShellApplication {
        name = "nix";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          [ "$1 $2" = "eval --json" ] || exit 1
          name=''${3#.#ci.}
          if [ "$name" = matrix ]; then
            cat "$MATRIX"
          else
            cat "$PLAN/$name.json"
          fi
        '';
      };

      fakeEvalJobs = pkgs.writeShellApplication {
        name = "nix-eval-jobs";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''cat "$EVALUATED"'';
      };

      ciPlan = import ./_ci-plan.nix {
        inherit pkgs;
        nix-eval-jobs = fakeEvalJobs;
      };

      test = pkgs.writeShellApplication {
        name = "plan-test";
        runtimeInputs = [
          fakeNix
          pkgs.jq
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.gnused
        ];
        runtimeEnv = {
          PLAN = ./_fixtures/plan;
          CI_PLAN = lib.getExe ciPlan;
        };
        excludeShellChecks = [ "SC2016" ];
        text = builtins.readFile ./plan-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        ci-plan = pkgs.runCommand "ci-plan" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
