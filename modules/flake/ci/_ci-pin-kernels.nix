{
  pkgs,
  nix,
  cachix,
}:

pkgs.writeShellApplication {
  name = "ci-pin-kernels";
  runtimeInputs = [
    nix
    cachix
    pkgs.jq
  ];
  text = builtins.readFile ./ci-pin-kernels.sh;
}
