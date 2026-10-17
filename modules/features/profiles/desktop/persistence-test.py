machine.succeed("findmnt -no SOURCE /var/lib/NetworkManager | grep -q /persist/var/lib/NetworkManager")
for ephemeral in ["/var/lib/cups", "/var/lib/power-profiles-daemon", "/var/lib/fwupd"]:
    machine.fail(f"findmnt {ephemeral}")
machine.succeed("test -e /var/lib/cups/cupsd.conf")
