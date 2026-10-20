{ inputs, ... }:

{
  apps.nix-index.nixos =
    { lib, pkgs, ... }:
    {
      imports = [ inputs.nix-index-database.nixosModules.default ];

      programs.nix-index.package =
        lib.mkOverride 900
          inputs.nix-index-database.packages.${pkgs.stdenv.hostPlatform.system}.nix-index-with-small-db;

      programs.nix-index-database.comma.enable = lib.mkDefault true;
    };
}
