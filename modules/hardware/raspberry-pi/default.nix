{ inputs, ... }:

let
  raspberrypi = inputs.nixos-raspberrypi;

  board =
    model:
    { lib, pkgs, ... }:
    {
      imports = [
        raspberrypi.lib.inject-overlays
        raspberrypi.nixosModules."raspberry-pi-${model}".base
        raspberrypi.nixosModules.sd-image
      ];

      _module.args.nixos-raspberrypi = raspberrypi;

      boot.kernelPackages = lib.mkOverride 900 pkgs."linuxPackages_rpi${model}";
      boot.loader.raspberry-pi.firmwarePackage = lib.mkOverride 900 pkgs.raspberrypifw;
      boot.supportedFilesystems.zfs = false;

      environment.systemPackages = [ pkgs.raspberrypi-eeprom ];
    };
in
{
  flake.modules.nixos = {
    raspberry-pi-4 = board "4";
    raspberry-pi-5 = board "5";
  };
}
