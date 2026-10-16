{ config, ... }:

let
  inherit (config.flake.modules) nixos;
  inherit (config) apps;
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

  helpers.baseCheck =
    module: testScripts:
    config.helpers.vmCheck "system-${module}" {
      nodes.machine.imports = [ nixos.base ];
      inherit testScripts;
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

  helpers.appCheck =
    app: testScripts:
    config.helpers.vmCheck "app-${app}" {
      nodes.machine = {
        imports = [
          nixos.base
          apps.${app}.nixos
        ];
        users.users.tester.isNormalUser = true;
        home-manager.users.tester = {
          imports = [ apps.${app}.homeManager ];
          home.stateVersion = "26.05";
        };
        virtualisation.memorySize = 2048;
      };
      inherit testScripts;
    };
}
