{
  base =
    {
      lib,
      pkgs,
      config,
      options,
      ...
    }:
    let
      wipedRoot = lib.attrByPath [ "disko" "enableConfig" ] false config;
      secureBoot = lib.attrByPath [ "boot" "lanzaboote" "enable" ] false config;

      btrbk = config.services.btrbk.instances.snapshots or null;
      btrbkVolume = if btrbk == null then { } else btrbk.settings.volume."/" or { };

      doctorConfig = {
        inherit secureBoot;
        luksDevice = if wipedRoot && secureBoot then config.forge.luksDevice else null;
        btrbkCalendar = if btrbk == null then null else btrbk.onCalendar;
        snapshotDir = "/${btrbkVolume.snapshot_dir or ""}";
        snapshotSubvolumes = lib.attrNames (btrbkVolume.subvolume or { });
        updateFlake = config.update.flake;
        autoUpgradeState =
          if config.systemd.services ? auto-upgrade then
            "/var/lib/${config.autoUpgrade.stateDirectory}"
          else
            null;
        deploySigners = lib.length config.update.signers;
        inherit wipedRoot;
        ephemeralPaths = config.doctor.ephemeralPaths;
      };

      doctorJson = pkgs.writeText "doctor.json" (builtins.toJSON doctorConfig);
    in
    {
      options.doctor.ephemeralPaths = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        example = [ "/root/.cache" ];
        description = "Paths on / whose loss at boot is expected: `forge doctor` leaves what they hold out of the files lost at the next boot. Modules add to the list.";
      };

      config = lib.mkMerge [
        {
          environment.etc."forge/doctor.json".source = doctorJson;

          forge.environment.FORGE_DOCTOR_CONFIG = "${doctorJson}";

          forge.runtimeInputs = [
            pkgs.coreutils
            pkgs.util-linux
            pkgs.git
            config.systemd.package
          ];

          doctor.ephemeralPaths = [
            "/tmp"
            "/var/tmp"
            "/var/lib/lastlog"
          ];
        }

        (lib.optionalAttrs (options ? disko) {
          disko.tests.extraConfig.systemd = {
            tmpfiles.rules = [ "f /var/lib/doctor-boot-state 0644 root root - written at boot" ];
            services.doctor-clock-step-back = {
              wantedBy = [ "sysinit.target" ];
              after = [ "systemd-tmpfiles-setup.service" ];
              before = [ "sysinit.target" ];
              unitConfig.DefaultDependencies = false;
              serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
              };
              script = ''
                ${lib.optionalString pkgs.stdenv.hostPlatform.isx86_64 "${pkgs.kmod}/bin/modprobe rtc_cmos"}
                stepped=$(($(date +%s%N) - 1500000000))
                date -s "@$((stepped / 1000000000)).$(printf %09d $((stepped % 1000000000)))"
              '';
            };
          };

          disko.tests.extraChecks = lib.mkMerge [
            (lib.mkOrder 1300 (builtins.readFile ./disk-test.py))
            (lib.mkIf (secureBoot && pkgs.stdenv.hostPlatform.isx86_64) (
              lib.mkOrder 1600 (builtins.readFile ./secure-boot-test.py)
            ))
          ];
        })
      ];
    };
}
