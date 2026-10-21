{ pkgs, nix }:

pkgs.writeShellApplication {
  name = "ci-flaky";
  runtimeInputs = [
    nix
    pkgs.jq
    pkgs.coreutils
    pkgs.gnugrep
  ];
  text = builtins.readFile ./ci-flaky.sh;
}
