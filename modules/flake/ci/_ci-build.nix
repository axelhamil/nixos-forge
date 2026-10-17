{ pkgs, nix }:

pkgs.writeShellApplication {
  name = "ci-build";
  runtimeInputs = [
    nix
    pkgs.jq
    pkgs.coreutils
    pkgs.gawk
  ];
  runtimeEnv.EXCERPT = ./excerpt.awk;
  text = builtins.readFile ./ci-build.sh;
}
