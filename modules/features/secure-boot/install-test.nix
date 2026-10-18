{ config, ... }:

let
  inherit (config.helpers) secureMachineScript;
in
{
  features.secure-boot =
    {
      lib,
      pkgs,
      options,
      ...
    }:
    {
      config = lib.optionalAttrs (options ? disko) {
        disko.tests.extraChecks = lib.mkIf pkgs.stdenv.hostPlatform.isx86_64 (
          lib.mkAfter ''
            ${secureMachineScript pkgs}
            ${builtins.readFile ./secure-boot-test.py}
          ''
        );
      };
    };
}
