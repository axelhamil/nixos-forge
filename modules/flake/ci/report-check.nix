{ lib, ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      ciBuildWith = nix: import ./_ci-build.nix { inherit pkgs nix; };

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

      test = pkgs.writeShellApplication {
        name = "report-test";
        runtimeInputs = [
          pkgs.gawk
          pkgs.jq
          pkgs.coreutils
          pkgs.gnugrep
        ];
        runtimeEnv = {
          EXCERPT = ./excerpt.awk;
          LOGS = ./_fixtures/logs;
          FLAKY_CI_BUILD = lib.getExe (ciBuildWith flakyNix);
          BROKEN_CI_BUILD = lib.getExe (ciBuildWith brokenNix);
        };
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
