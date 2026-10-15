{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  helpers.profileCheck =
    profile: testScripts:
    {
      lib,
      pkgs,
      system,
      ...
    }:
    lib.mkIf (system == "x86_64-linux") {
      checks."profile-${profile}" = pkgs.testers.runNixOSTest {
        name = "profile-${profile}";
        nodes.machine = {
          imports = [
            nixos.base
            nixos.${profile}
          ];
          virtualisation.cores = 2;
          system.stateVersion = "26.05";
        };
        testScript = lib.concatMapStrings builtins.readFile testScripts;
      };
    };
}
