{ config, ... }:

{
  perSystem = config.helpers.featureCheck "docker" {
    machine = {
      virtualisation.emptyDiskImages = [ 2048 ];
      virtualisation.fileSystems."/home" = {
        device = "/dev/vdb";
        fsType = "btrfs";
        autoFormat = true;
      };
      users.users.tester = {
        isNormalUser = true;
        uid = 1000;
      };
    };
    testScripts = [ ./test.py ];
  };
}
