{
  features.plymouth =
    { lib, ... }:
    {
      boot.plymouth = {
        enable = true;
        theme = lib.mkDefault "bgrt";
      };

      boot.consoleLogLevel = lib.mkOverride 900 3;
      boot.kernelParams = [
        "quiet"
        "udev.log_level=3"
        "systemd.show_status=auto"
      ];

      doctor.ephemeralPaths = [ "/var/lib/plymouth" ];
    };
}
