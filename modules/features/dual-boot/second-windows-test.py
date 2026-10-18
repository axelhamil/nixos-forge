with subtest("a second Windows disk, plugged in then removed, never replaces the Windows boot manager of the shared ESP"):
    shared = machine.succeed("sha256sum < /boot/EFI/Microsoft/Boot/bootmgfw.efi")
    machine.shutdown()
    machine = start_uefi_machine("second_windows_machine", *extra_disk(f"{state}/second-windows.qcow2"))
    machine.succeed(f"{fake_windows} /dev/vdb 256M 0")
    machine.succeed("dir=$(mktemp -d) && mount /dev/vdb1 $dir && echo other-build >> $dir/EFI/Microsoft/Boot/bootmgfw.efi && umount $dir && rmdir $dir")
    machine.succeed("systemctl restart dual-boot")
    assert machine.succeed("sha256sum < /boot/EFI/Microsoft/Boot/bootmgfw.efi") == shared, "the service replaced the Windows boot manager with the one of another disk"
    machine.fail("test -e /boot/EFI/Microsoft/Boot/copied-by-nixos")
    machine.shutdown()
    machine = start_uefi_machine("beside_again_machine")
    machine.succeed("systemctl restart dual-boot")
    assert machine.succeed("sha256sum < /boot/EFI/Microsoft/Boot/bootmgfw.efi") == shared, "the service removed the Windows boot manager once the other disk was gone"
