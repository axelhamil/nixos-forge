{ config, ... }:

{
  configurations.nixos.example.module = {
    imports = with config.flake.modules.nixos; [
      base
      disk
      secure-boot
      firmware
      audio
      bluetooth
      desktop
      plymouth
      hyprland
      alice
    ];

    networking.hostName = "example";

    boot.loader.systemd-boot.enable = true;
    boot.loader.efi.canTouchEfiVariables = true;

    time.timeZone = "UTC";
    i18n.defaultLocale = "en_US.UTF-8";
    console.keyMap = "us";
    services.xserver.xkb.layout = "us";

    virtualisation.vmVariant.virtualisation = {
      memorySize = 8192;
      cores = 4;
    };

    system.stateVersion = "26.05";
  };
}
