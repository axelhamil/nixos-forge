machine.wait_for_unit("multi-user.target", timeout=timedelta(minutes=5))
machine.succeed("findmnt -no OPTIONS /.snapshots | grep -q subvol=/@snapshots")
machine.succeed("systemctl stop btrbk-snapshots.timer btrbk-snapshots.service")
machine.succeed("touch /home/snapshot-marker /persist/snapshot-marker", "sync")
machine.succeed("systemctl start btrbk-snapshots.service")
machine.succeed("ls /.snapshots/persist.*/snapshot-marker")

if home_snapshotted:
    machine.succeed("ls /.snapshots/home.*/snapshot-marker")
else:
    machine.fail("ls -d /.snapshots/home.*")
