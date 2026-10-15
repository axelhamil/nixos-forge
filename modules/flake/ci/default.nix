{ lib, config, ... }:

{
  flake.ci.checks = lib.filter (name: name != "treefmt") (
    lib.attrNames config.flake.checks.x86_64-linux
  );

  perSystem =
    { pkgs, ... }:
    {
      packages.closure-diff = pkgs.writeShellApplication {
        name = "closure-diff";
        runtimeInputs = [ pkgs.nvd ];
        text = builtins.readFile ./closure-diff.sh;
      };

      packages.sync-develop = pkgs.writeShellApplication {
        name = "sync-develop";
        runtimeInputs = [ pkgs.git ];
        text = builtins.readFile ./sync-develop.sh;
      };
    };
}
