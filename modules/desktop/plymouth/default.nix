{
  flake.modules.nixos.plymouth =
    { lib, ... }:
    {
      boot.plymouth = {
        enable = true;
        theme = lib.mkDefault "bgrt";
      };

      boot.consoleLogLevel = lib.mkDefault 3;
      boot.initrd.verbose = lib.mkDefault false;
      boot.kernelParams = [
        "quiet"
        "udev.log_level=3"
        "systemd.show_status=auto"
      ];
    };
}
