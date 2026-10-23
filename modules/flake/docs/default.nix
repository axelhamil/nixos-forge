{ lib, self, ... }:

{
  perSystem =
    {
      pkgs,
      config,
      system,
      ...
    }:
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        docs = pkgs.runCommand "docs" { } ''
          ${lib.getExe config.packages.forge} check-docs ${self}
          touch $out
        '';
      };
    };
}
