{ lib, config, ... }:

let
  inherit (config.helpers)
    hostsOn
    hasDisk
    testRevision
    storeOnDiskTest
    diskTest
    layerMachine
    ;
  inherit (config) features;

  hostConfig = name: config.flake.nixosConfigurations.${name};

  testSwap.hibernate.swapSize = lib.mkForce "1G";

  hibernatesOnDisk = name: hasDisk name && (hostConfig name).options ? hibernate;

  resumeTest =
    { lib, ... }:
    {
      imports = [ storeOnDiskTest ];

      disko.tests.extraConfig = {
        powerManagement.powerDownCommands = "systemctl --no-block stop backdoor.service";
        powerManagement.resumeCommands = "systemctl --no-block restart backdoor.service";
      };

      disko.tests.extraChecks = lib.mkAfter (builtins.readFile ./resume-test.py);
    };

  hibernatingMachine =
    system:
    layerMachine system [
      features.disk
      features.hibernate
      testSwap
      resumeTest
      testRevision
      {
        networking.hostName = "hibernate";
        zramSwap.enable = true;
      }
    ];
in
{
  perSystem =
    { system, ... }:
    {
      checks =
        lib.concatMapAttrs (name: _: {
          "${name}-disk" = lib.mkForce (
            diskTest 1500
              ((hostConfig name).extendModules {
                modules = [
                  testSwap
                  testRevision
                ];
              }).config.system.build.installTest
          );
        }) (lib.filterAttrs (name: _: hibernatesOnDisk name) (hostsOn system))
        // lib.optionalAttrs (system == "x86_64-linux") {
          feature-hibernate = diskTest 1300 (hibernatingMachine system).config.system.build.installTest;
        };
    };
}
