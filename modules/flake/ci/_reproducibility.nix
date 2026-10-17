{ pkgs, nix }:

pkgs.writeShellApplication {
  name = "reproducibility";
  runtimeInputs = [
    nix
    pkgs.jq
    pkgs.coreutils
    pkgs.diffoscopeMinimal
  ];
  excludeShellChecks = [ "SC2016" ];
  text = builtins.readFile ./reproducibility.sh;
}
