{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "dev-libvirt" {
    nodes.machine =
      { pkgs, ... }:
      {
        imports = [
          nixos.base
          nixos.libvirt
        ];
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
