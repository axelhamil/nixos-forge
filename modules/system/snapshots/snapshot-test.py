machine.wait_for_unit("multi-user.target")
machine.succeed("findmnt -no OPTIONS /.snapshots | grep -q subvol=/@snapshots")
machine.succeed("touch /home/snapshot-marker /persist/snapshot-marker")
machine.succeed("systemctl start btrbk-snapshots.service")
machine.succeed("ls /.snapshots/persist.*/snapshot-marker")

if home_snapshotted:
    machine.succeed("ls /.snapshots/home.*/snapshot-marker")
else:
    machine.fail("ls -d /.snapshots/home.*")
