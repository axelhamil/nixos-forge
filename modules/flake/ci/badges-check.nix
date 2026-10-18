{ lib, ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      test = pkgs.writeShellApplication {
        name = "badges-test";
        runtimeInputs = [
          pkgs.jq
          pkgs.coreutils
          pkgs.git
        ];
        runtimeEnv = {
          RUNS = ./_fixtures/runs;
          CI_BADGES = lib.getExe (import ./_ci-badges.nix { inherit pkgs; });
        };
        excludeShellChecks = [ "SC2016" ];
        text = builtins.readFile ./badges-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        ci-badges = pkgs.runCommand "ci-badges" { } ''
          cd "$TMPDIR"
          export HOME=$TMPDIR
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
