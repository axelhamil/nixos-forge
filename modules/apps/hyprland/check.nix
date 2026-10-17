{ config, ... }:

{
  perSystem = config.helpers.appCheck "hyprland" {
    machine = {
      users.users.tester.password = "test";
      virtualisation = {
        memorySize = 4096;
        cores = 2;
        qemu.options = [ "-vga none -device virtio-gpu-pci" ];
      };
    };
    testScripts = [ ./test.py ];
  };
}
