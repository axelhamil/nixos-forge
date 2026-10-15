{
  flake.modules.nixos.disk =
    { lib, ... }:
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
              home = { };
              persist = { };
            };
          };
        };
      };

      systemd.services.btrbk-snapshots.unitConfig.RequiresMountsFor = "/.snapshots";

      disko.tests.extraChecks = lib.mkOrder 1200 (builtins.readFile ./snapshot-test.py);

      virtualisation.vmVariant.services.btrbk.instances = lib.mkForce { };
    };
}
