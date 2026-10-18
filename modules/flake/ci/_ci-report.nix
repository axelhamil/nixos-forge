{ pkgs }:

pkgs.writeShellApplication {
  name = "ci-report";
  runtimeInputs = [
    pkgs.jq
    pkgs.coreutils
    pkgs.gawk
    (import ./narvi/_package.nix { inherit pkgs; })
  ];
  runtimeEnv = {
    CAUSES = ./causes.tsv;
    TRUNCATE = ./truncate.awk;
  };
  excludeShellChecks = [ "SC2016" ];
  text = builtins.readFile ./ci-report.sh;
}
