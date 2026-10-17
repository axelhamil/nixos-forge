{
  features.disk =
    { lib, config, ... }:
    let
      homeSnapshotted = !config.impermanence.wipeHome;
    in
    {
      services.btrbk.instances.snapshots = {
        onCalendar = "hourly";
        snapshotOnly = true;
        settings = {
          snapshot_create = "onchange";
          snapshot_preserve_min = "2d";
          snapshot_preserve = "14d 4w";
          timestamp_format = "long";
          volume."/" = {
            snapshot_dir = ".snapshots";
            subvolume = {
              persist = { };
            }
            // lib.optionalAttrs homeSnapshotted { home = { }; };
          };
        };
      };

      systemd.services.btrbk-snapshots.unitConfig.RequiresMountsFor = "/.snapshots";

      doctor.ephemeralPaths = [ "/var/lib/btrbk/.ssh" ];

      disko.tests.extraChecks = lib.mkOrder 1200 ''
        home_snapshotted = ${if homeSnapshotted then "True" else "False"}
        ${builtins.readFile ./snapshot-test.py}
      '';

      virtualisation.vmVariant.services.btrbk.instances = lib.mkForce { };
    };
}
