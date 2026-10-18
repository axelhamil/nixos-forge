machine.succeed("findmnt -no SOURCE /var/lib/NetworkManager | grep -q /persist/var/lib/NetworkManager")
machine.fail("findmnt /var/lib/power-profiles-daemon")
