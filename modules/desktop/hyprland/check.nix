{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "desktop-hyprland" {
    nodes.machine = {
      imports = [
        nixos.base
        nixos.hyprland
      ];
      users.users.tester = {
        isNormalUser = true;
        password = "test";
      };
      home-manager.users.tester.home.stateVersion = "26.05";
      virtualisation = {
        memorySize = 4096;
        cores = 2;
        qemu.options = [ "-vga none -device virtio-gpu-pci" ];
      };
    };
    testScripts = [ ./test.py ];
  };
}
