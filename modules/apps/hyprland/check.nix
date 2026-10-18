{ config, ... }:

let
  inherit (config.helpers) appPart;

  desktopApps = [
    "waybar"
    "fuzzel"
    "kitty"
    "mako"
    "lock"
  ];
in
{
  perSystem = config.helpers.appCheck "hyprland" {
    machine = {
      imports = [
        config.features.greetd
        (appPart "lock" "nixos")
      ];
      users.users.tester.password = "test";
      home-manager.users.tester.imports = map (app: appPart app "homeManager") desktopApps;
      virtualisation = {
        memorySize = 4096;
        cores = 2;
        qemu.options = [ "-vga none -device virtio-gpu-pci" ];
      };
    };
    testScripts = [ ./test.py ];
  };
}
