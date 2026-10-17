{ lib, ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      ciBuildWith = nix: import ./_ci-build.nix { inherit pkgs nix; };

      checks = import ./_checks.nix { inherit lib; };

      hostChecks = lib.importJSON ./_fixtures/runs/host-checks.json;

      descriptions = lib.genAttrs (
        map (job: job.check) (lib.importJSON ./_fixtures/runs/planned.json) ++ [ "treefmt" ]
      ) (checks.describe hostChecks);

      ciReport = import ./_ci-report.nix { inherit pkgs; };

      fakeNix =
        text:
        pkgs.writeShellApplication {
          name = "nix";
          runtimeInputs = [ pkgs.coreutils ];
          text = ''
            echo "$*" >> "$NIX_CALLS"
            ${text}
          '';
        };

      flakyNix = fakeNix ''
        if [ "$(wc -l < "$NIX_CALLS")" -eq 1 ]; then
          cat ${./_fixtures/logs/timeout.log}
          exit 1
        fi
        echo "built"
      '';

      brokenNix = fakeNix ''
        cat ${./_fixtures/logs/eval-error.log}
        exit 1
      '';

      killedNix = fakeNix ''
        if [ "$(wc -l < "$NIX_CALLS")" -eq 1 ]; then
          cat ${./_fixtures/logs/timeout.log}
          exit 1
        fi
        kill -KILL "$PPID"
        exit 1
      '';

      test = pkgs.writeShellApplication {
        name = "report-test";
        runtimeInputs = [
          pkgs.gawk
          pkgs.jq
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.gnused
        ];
        runtimeEnv = {
          EXCERPT = ./excerpt.awk;
          LOGS = ./_fixtures/logs;
          RUNS = ./_fixtures/runs;
          CI_REPORT = lib.getExe ciReport;
          DESCRIPTIONS_FILE = pkgs.writeText "descriptions.json" (builtins.toJSON descriptions);
          FLAKY_CI_BUILD = lib.getExe (ciBuildWith flakyNix);
          BROKEN_CI_BUILD = lib.getExe (ciBuildWith brokenNix);
          KILLED_CI_BUILD = lib.getExe (ciBuildWith killedNix);
        };
        excludeShellChecks = [ "SC2016" ];
        text = builtins.readFile ./report-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        ci-report = pkgs.runCommand "ci-report" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
