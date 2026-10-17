{ pkgs }:

pkgs.writeShellApplication {
  name = "ci-badges";
  runtimeInputs = [
    pkgs.jq
    pkgs.coreutils
    pkgs.gnugrep
    pkgs.git
  ];
  excludeShellChecks = [ "SC2016" ];
  text = builtins.readFile ./ci-badges.sh;
}
