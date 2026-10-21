{
  features.disk =
    { lib, config, ... }:
    let
      homeSnapshotted = !config.impermanence.wipeHome;
    in
    {
      services.btrbk.instances.snapshots = {
        onCalendar = lib.mkDefault "hourly";
        snapshotOnly = true;
        settings = {
          snapshot_create = lib.mkDefault "onchange";
          snapshot_preserve_min = lib.mkDefault "2d";
          snapshot_preserve = lib.mkDefault "14d 4w";
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

      disko.tests.extraChecks = lib.mkOrder 1200 ''
        home_snapshotted = ${if homeSnapshotted then "True" else "False"}
        ${builtins.readFile ./snapshot-test.py}
      '';

      virtualisation.vmVariant.services.btrbk.instances = lib.mkForce { };
    };
}
