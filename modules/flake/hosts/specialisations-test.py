person = machine.succeed("ls /home").split()[0]
home_file = f"/home/{person}/variant"


def in_variant(present):
    check = machine.succeed if present else machine.fail
    check("test -x /run/current-system/sw/bin/hello")
    check("grep -q NIX_LD /etc/set-environment")
    check("test -e /etc/gamemode.ini")
    check(f"id -nG {person} | grep -qw gamemode")
    check("test -e /etc/systemd/system/bluetooth.service")
    check("grep -q VARIANT_MODULE /etc/set-environment")
    check(f"test -e {home_file}")
    check(f"grep -q output_folder /home/{person}/.config/MangoHud/MangoHud.conf")


machine.wait_for_unit("multi-user.target")
machine.wait_for_unit(f"home-manager-{person}.service")
machine.fail("test -e /etc/specialisation")
in_variant(False)
machine.fail("test -c /dev/ntsync")

machine.succeed("/run/current-system/specialisation/gaming/bin/switch-to-configuration test")
assert machine.succeed("cat /etc/specialisation") == "gaming"
in_variant(True)
machine.succeed("test -c /dev/ntsync")

machine.succeed("/run/booted-system/bin/switch-to-configuration test")
machine.fail("test -e /etc/specialisation")
in_variant(False)
