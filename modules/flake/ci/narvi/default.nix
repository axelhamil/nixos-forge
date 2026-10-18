{
  perSystem =
    { pkgs, ... }:
    {
      packages.narvi = import ./_package.nix { inherit pkgs; };
    };
}
