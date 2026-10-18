{ config, ... }:

{
  perSystem = config.helpers.featureCheck "libvirt" {
    machine =
      { pkgs, ... }:
      {
        boot.initrd.supportedFilesystems = [ "btrfs" ];
        virtualisation.emptyDiskImages = [ 4096 ];
        virtualisation.useDefaultFilesystems = false;
        virtualisation.fileSystems."/" = {
          device = "/dev/vdb";
          fsType = "btrfs";
          autoFormat = true;
        };
        environment.systemPackages = [ pkgs.e2fsprogs ];
      };
    testScripts = [ ./test.py ];
  };
}
