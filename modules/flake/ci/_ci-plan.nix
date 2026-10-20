{ pkgs, nix-eval-jobs }:

pkgs.writeShellApplication {
  name = "ci-plan";
  runtimeInputs = [
    nix-eval-jobs
    pkgs.jq
    pkgs.coreutils
  ];
  text = builtins.readFile ./ci-plan.sh;
}
