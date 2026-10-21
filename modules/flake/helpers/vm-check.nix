{ lib, config, ... }:

let
  inherit (config.helpers) appPart;
  inherit (config)
    base
    profiles
    features
    ;

  machineCheck =
    name: modules:
    {
      machine ? { },
      ...
    }@check:
    config.helpers.vmCheck name (
      removeAttrs check [ "machine" ]
      // {
        nodes.machine.imports = [ base ] ++ modules ++ [ machine ];
      }
    );
in
{
  helpers.testRevision = {
    system.configurationRevision = lib.mkForce "test";
  };

  helpers.vmCheck =
    name:
    {
      nodes,
      qemu ? null,
      globalTimeout ? 400,
      testScripts,
    }:
    {
      lib,
      pkgs,
      system,
      ...
    }:
    lib.mkIf (system == "x86_64-linux") {
      checks.${name} = pkgs.testers.runNixOSTest {
        inherit name nodes globalTimeout;
        qemu = lib.optionalAttrs (qemu != null) { package = qemu pkgs; };
        defaults = {
          imports = [ config.helpers.testRevision ];
          system.stateVersion = "26.05";
        };
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
    app:
    {
      machine ? { },
      ...
    }@check:
    config.helpers.vmCheck "app-${app}" (
      removeAttrs check [ "machine" ]
      // {
        nodes.machine =
          { lib, ... }:
          {
            imports = [
              base
              (appPart app "nixos")
              machine
            ];
            users.users.tester.isNormalUser = true;
            home-manager.users.tester = {
              imports = [ (appPart app "homeManager") ];
              home.stateVersion = "26.05";
            };
            virtualisation.memorySize = lib.mkDefault 2048;
          };
      }
    );
}
