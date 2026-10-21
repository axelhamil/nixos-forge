import re

boot_manager = "Windows Boot Manager"
detour = "NixOS again"
bitlocker_volume = f"{windows_disk}4"


def firmware_entries():
    listing = machine.succeed("efibootmgr")
    entries = {m[2]: m[1] for m in re.finditer(r"^Boot([0-9A-F]{4})\*? ([^\t\n]+)", listing, re.M)}
    order_line = re.search(r"^BootOrder: (\S*)", listing, re.M)
    assert order_line, f"no BootOrder: {listing}"
    order = order_line[1].split(",")
    return entries, order


def check_boot_order():
    entries, order = firmware_entries()
    assert boot_manager in entries, f"no {boot_manager} firmware entry: {entries}"
    expected = [entries["Linux Boot Manager"], entries[boot_manager]]
    assert order[:2] == expected, f"boot order {order} does not start with NixOS then Windows {expected}"
    return entries[boot_manager]


def pick_windows():
    since = len(machine.full_console_log)
    before = windows_digest()
    machine.succeed("systemctl --no-block reboot --boot-loader-entry=auto-windows")
    machine.connected = False
    return since, before


def console_since(since):
    return re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", "", "\n".join(machine.full_console_log[since:]))


def back_in_nixos(before):
    wait_for_nixos()
    machine.wait_for_unit("dual-boot.service", timeout=timedelta(minutes=5))
    assert windows_digest() == before, "starting Windows from the menu changed the Windows partitions"


def windows_started(since, entry):
    machine.wait_for_console_text("fake-windows-done", timeout=timedelta(minutes=10))
    console = console_since(since)
    secure_boot = re.search(r"SecureBoot' DataSize = 0x01.*?00000000: ([0-9A-F]{2})", console, re.S)
    assert secure_boot and secure_boot[1] == expected_secure_boot, f"Windows did not start with SecureBoot {expected_secure_boot}: {console[-600:]!r}"
    current = re.search(r"BootCurrent' DataSize = 0x02.*?00000000: ([0-9A-F]{2}) ([0-9A-F]{2})", console, re.S)
    assert current and current[2] + current[1] == entry, f"the firmware did not start Windows from {boot_manager} Boot{entry}: {console[-600:]!r}"


def detour_before_windows():
    entries, order = firmware_entries()
    if detour not in entries:
        machine.succeed(
            "esp=$(findmnt -nvo SOURCE /boot) && "
            "efibootmgr --create-only --disk /dev/$(lsblk -ndo PKNAME $esp) --part $(cat /sys/class/block/$(basename $esp)/partition) "
            f"--label '{detour}' --loader '\\EFI\\systemd\\systemd-bootx64.efi' >&2"
        )
        entries, order = firmware_entries()
    linux, windows, other = entries["Linux Boot Manager"], entries[boot_manager], entries[detour]
    rest = [e for e in order if e not in (linux, windows, other)]
    machine.succeed(f"efibootmgr --bootorder {','.join([linux, other, windows, *rest])} >&2")
    return other


with subtest("Windows is in the boot menu, after NixOS in the firmware boot order"):
    machine.wait_for_unit("dual-boot.service", timeout=timedelta(minutes=5))
    machine.succeed("test -s /boot/EFI/Microsoft/Boot/bootmgfw.efi")
    machine.succeed("grep -Eqx 'reboot-for-bitlocker (true|yes)' /boot/loader/loader.conf")
    entries = machine.succeed("bootctl list --json=short --no-pager")
    assert '"id":"auto-windows"' in entries, entries
    windows_entry = check_boot_order()

with subtest("NixOS reads the Windows partition, read-only"):
    assert machine.succeed("cat /mnt/windows/Users/Public/hello.txt").strip() == "hello from windows"
    assert "ro" in machine.succeed("findmnt -no OPTIONS --types ntfs3 --mountpoint /mnt/windows").strip().split(","), "the Windows partition is mounted read-write"
    machine.fail("touch /mnt/windows/Users/Public/written-by-nixos")

with subtest("the file manager mounts an NTFS partition read-only"):
    machine.succeed(f"truncate -s 16M /tmp/ntfs.img && {mkntfs} -F -Q -L Data /tmp/ntfs.img >&2")
    loop = re.search(r" as (/dev/loop\d+)", machine.succeed("udisksctl loop-setup --no-user-interaction -f /tmp/ntfs.img"))
    assert loop, "udisks did not set up the NTFS image"
    mounted = machine.succeed(f"udisksctl mount --no-user-interaction -b {loop[1]}")
    mount_point = re.search(r" at (\S+)", mounted)
    assert mount_point, mounted
    path = mount_point[1].rstrip(".")
    options = machine.succeed(f"findmnt -no FSTYPE,OPTIONS --mountpoint {path}").split()
    assert options[0] == "ntfs3" and "ro" in options[1].split(","), f"udisks mounts NTFS read-write: {options}"
    machine.succeed(f"udisksctl unmount --no-user-interaction -b {loop[1]} && udisksctl loop-delete --no-user-interaction -b {loop[1]}")

with subtest("with BitLocker, the menu entry sets BootNext, so the firmware starts Windows whatever its boot order"):
    machine.succeed(f"printf '\\353\\130\\220-FVE-FS-' | dd of={bitlocker_volume} conv=notrunc status=none && sync")
    detour_before_windows()
    since, before = pick_windows()
    windows_started(since, windows_entry)
    back_in_nixos(before)
    machine.fail("efibootmgr | grep -q '^BootNext'")
    windows_entry = check_boot_order()

with subtest("without BitLocker, systemd-boot gives up the entry and the firmware boots its next entry"):
    machine.succeed(f"dd if=/dev/zero of={bitlocker_volume} bs=512 count=1 conv=notrunc status=none && sync")
    other = detour_before_windows()
    since, before = pick_windows()
    back_in_nixos(before)
    assert "fake-windows-started" not in console_since(since), "Windows started although the boot order put NixOS next"
    current = re.search(r"^BootCurrent: (\S+)", machine.succeed("efibootmgr"), re.M)
    assert current and current[1] == other, f"the firmware did not boot the entry after systemd-boot, Boot{other}"
    windows_entry = check_boot_order()

with subtest("without BitLocker, with the order the service keeps, the firmware starts Windows next"):
    since, before = pick_windows()
    windows_started(since, windows_entry)
    back_in_nixos(before)

with subtest("without a Windows Boot Manager firmware entry, the firmware goes on to NixOS and the next boot recreates the entry"):
    machine.succeed(f"printf '\\353\\130\\220-FVE-FS-' | dd of={bitlocker_volume} conv=notrunc status=none && sync")
    machine.succeed(f"efibootmgr --bootnum {windows_entry} --delete-bootnum >&2")
    since, before = pick_windows()
    back_in_nixos(before)
    assert "fake-windows-started" not in console_since(since), "Windows started without its firmware entry"
    windows_entry = check_boot_order()
    machine.succeed("journalctl -b -u dual-boot -o cat | grep -q 'created the firmware entry Windows Boot Manager'")
