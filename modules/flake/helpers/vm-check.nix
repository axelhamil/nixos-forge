{ config, ... }:

let
  inherit (config)
    apps
    base
    profiles
    features
    ;

  machineCheck =
    name: modules:
    {
      machine ? { },
      testScripts,
    }:
    config.helpers.vmCheck name {
      nodes.machine.imports = [ base ] ++ modules ++ [ machine ];
      inherit testScripts;
    };
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

  helpers.baseCheck = tool: machineCheck "base-${tool}" [ ];

  helpers.featureCheck = feature: machineCheck "feature-${feature}" [ features.${feature} ];

  helpers.profileCheck =
    profile:
    machineCheck "profile-${profile}" [
      profiles.${profile}
      { virtualisation.cores = 2; }
    ];

  helpers.appCheck =
    app: testScripts:
    config.helpers.vmCheck "app-${app}" {
      nodes.machine = {
        imports = [
          base
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
