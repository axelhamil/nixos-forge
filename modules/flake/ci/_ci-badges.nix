{ pkgs, forge-ci }:

pkgs.writeShellApplication {
  name = "ci-badges";
  runtimeInputs = [
    forge-ci
    pkgs.git
  ];
  text = ''exec forge-ci badges "$@"'';
}
