machine.shutdown()
machine = start_uefi_machine("dual_boot_machine", *extra_disk(f"{state}/windows.qcow2"))

windows_disk = "/dev/vdb"
windows_content = [1, 2, 3, 4]
machine.succeed("test \"$(lsblk -no PKNAME /dev/disk/by-partlabel/disk-main-ESP)\" = vda")

with subtest("a Windows disk gets its boot manager copied onto the NixOS ESP, read-only"):
    machine.succeed(f"{fake_windows} {windows_disk} 256M 0")
    machine.succeed(f"efibootmgr --create --disk {windows_disk} --part 1 --label 'Windows Boot Manager' --loader '\\EFI\\Microsoft\\Boot\\bootmgfw.efi' >&2")
    created = windows_digest()
    machine.succeed("systemctl restart dual-boot")
    machine.succeed(
        f"dir=$(mktemp -d) && mount -o ro {windows_disk}1 $dir && "
        "cmp $dir/EFI/Microsoft/Boot/bootmgfw.efi /boot/EFI/Microsoft/Boot/bootmgfw.efi; status=$?; umount $dir; exit $status"
    )
    machine.succeed("test -e /boot/EFI/Microsoft/Boot/copied-by-nixos")
    machine.fail("test -e /boot/EFI/Microsoft/Boot/startup.nsh")
    assert windows_digest() == created, "the copy wrote to the Windows disk"
    machine.succeed("systemctl --no-block reboot")
    machine.connected = False
    wait_for_nixos()
