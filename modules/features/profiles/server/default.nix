{
  profiles.server = {
    services.tuned = {
      enable = true;
      ppdSupport = false;
    };

    doctor.ephemeralPaths = [
      "/etc/tuned"
      "/etc/modprobe.d/tuned.conf"
    ];
  };
}
