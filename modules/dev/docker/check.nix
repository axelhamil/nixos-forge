{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "dev-docker" {
    nodes.machine = {
      imports = [
        nixos.base
        nixos.docker
      ];
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
