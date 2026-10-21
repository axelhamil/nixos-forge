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

      doctor = pkgs.writeShellApplication {
        name = "doctor";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.findutils
          pkgs.gawk
          pkgs.gnugrep
          pkgs.gnused
          pkgs.util-linux
          pkgs.git
          pkgs.jq
          config.systemd.package
        ];
        runtimeEnv = {
          secure_boot = lib.boolToString secureBoot;
          luks_device = lib.optionalString (
            wipedRoot && secureBoot
          ) config.disko.devices.disk.main.content.partitions.luks.device;
          btrbk_calendar = if btrbk == null then "" else btrbk.onCalendar;
          snapshot_dir = "/${btrbkVolume.snapshot_dir or ""}";
          snapshot_subvolumes = lib.attrNames (btrbkVolume.subvolume or { });
          update_flake = lib.optionalString (config.update.flake != null) config.update.flake;
          auto_upgrade_state = lib.optionalString (
            config.systemd.services ? auto-upgrade
          ) "/var/lib/auto-upgrade";
          deploy_signers = toString (lib.length config.update.signers);
          wiped_root = lib.boolToString wipedRoot;
          ephemeral_paths = config.doctor.ephemeralPaths;
        };
        text = builtins.readFile ./doctor.sh;
      };
    in
    {
      options.doctor.ephemeralPaths = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        example = [ "/root/.cache" ];
        description = "Paths on / whose loss at boot is expected: `doctor` leaves what they hold out of the files lost at the next boot. Modules add to the list.";
      };

      config = lib.mkMerge [
        {
          environment.systemPackages = [ doctor ];

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
