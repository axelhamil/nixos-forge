{ config, ... }:

{
  perSystem = config.helpers.featureCheck "plymouth" {
    machine =
      { lib, pkgs, ... }:
      {
        virtualisation = {
          emptyDiskImages = [ 512 ];
          useBootLoader = true;
          useEFIBoot = true;
          mountHostNixStore = true;
        };
        boot.loader.systemd-boot.enable = true;
        environment.systemPackages = [ pkgs.cryptsetup ];

        specialisation.encrypted.configuration = {
          boot.initrd.luks.devices = lib.mkVMOverride { cryptroot.device = "/dev/vdb"; };
          virtualisation.rootDevice = "/dev/mapper/cryptroot";
        };
      };
    testScripts = [ ./test.py ];
  };
}
