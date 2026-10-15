{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  helpers.vmCheck =
    name:
    { nodes, testScripts }:
    {
      lib,
      pkgs,
      system,
      ...
    }:
    lib.mkIf (system == "x86_64-linux") {
      checks.${name} = pkgs.testers.runNixOSTest {
        inherit name nodes;
        defaults.system.stateVersion = "26.05";
        testScript = lib.concatMapStrings builtins.readFile testScripts;
      };
    };

  helpers.profileCheck =
    profile: testScripts:
    config.helpers.vmCheck "profile-${profile}" {
      nodes.machine = {
        imports = [
          nixos.base
          nixos.${profile}
        ];
        virtualisation.cores = 2;
      };
      inherit testScripts;
    };
}
