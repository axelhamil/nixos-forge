{
  features.printing = {
    services.printing.enable = true;

    services.avahi = {
      enable = true;
      nssmdns4 = true;
    };

    doctor.ephemeralPaths = [
      "/etc/printcap"
      "/var/lib/cups"
      "/var/cache/cups"
    ];
  };
}
