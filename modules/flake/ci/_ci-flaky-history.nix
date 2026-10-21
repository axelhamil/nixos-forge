{ pkgs, nix }:

let
  history = pkgs.writeShellApplication {
    name = "flaky-history";
    runtimeInputs = [
      (import ./_ci-badges.nix { inherit pkgs; })
      pkgs.jq
    ];
    text = builtins.readFile ./flaky-history.sh;
  };
in
{
  plan = pkgs.writeShellApplication {
    name = "ci-flaky-plan";
    runtimeInputs = [
      nix
      history
      pkgs.git
      pkgs.jq
    ];
    text = builtins.readFile ./ci-flaky-plan.sh;
  };

  report = pkgs.writeShellApplication {
    name = "ci-flaky-report";
    runtimeInputs = [
      history
      pkgs.git
      pkgs.jq
      pkgs.coreutils
    ];
    runtimeEnv.REPORT = ./flaky-report.jq;
    excludeShellChecks = [ "SC2016" ];
    text = builtins.readFile ./ci-flaky-report.sh;
  };
}
