machine.succeed("findmnt -no SOURCE /var/lib/cups | grep -q /persist/var/lib/cups")
machine.succeed("test -e /persist/var/lib/cups/cupsd.conf")
