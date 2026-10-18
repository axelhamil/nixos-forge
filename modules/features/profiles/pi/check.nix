{ config, ... }:

{
  perSystem = config.helpers.profileCheck "pi" {
    machine.nix.buildMachines = [
      {
        hostName = "builder.example";
        system = "aarch64-linux";
        sshUser = "builder";
      }
    ];
    testScripts = [
      ../server/test.py
      ./test.py
    ];
  };
}
