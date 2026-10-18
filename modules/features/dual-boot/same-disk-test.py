import subprocess

machine.shutdown()
installer = next(m for m in driver.machines_qemu if m.name == "machine")
subprocess.run([qemu_img, "resize", "-q", f"{installer.state_dir}/empty0.qcow2", "12G"], check=True)
installer.start()
installer.wait_for_unit("multi-user.target")
installer.succeed(f"printf %s {test_passphrase} | {cryptsetup} open --key-file=- /dev/disk/by-partlabel/disk-main-luks old")
installer.succeed("mkdir -p /old && mount -o subvol=@nix /dev/mapper/old /old")
installed_system = installer.succeed("readlink /old/var/nix/profiles/system-1-link").strip()
installer.succeed(f"umount /old && {cryptsetup} close old")

windows_disk = "/dev/vdb"
windows_content = [2, 3, 4]

with subtest("an ESP too small for NixOS stops the install and changes nothing"):
    installer.succeed(f"{fake_windows} {windows_disk} 100M 40")
    untouched = installer.succeed(f"sha256sum {windows_disk}")
    output = installer.fail(f"{install_beside_windows} {windows_disk} {installed_system} 2>&1")
    print(output)
    assert "Grow the ESP to 1 GiB" in output, output
    assert installer.succeed(f"sha256sum {windows_disk}") == untouched, "a failed install changed the disk"

with subtest("NixOS installs in the free space beside Windows, over an old LUKS header, and shares its ESP"):
    installer.succeed(f"{fake_windows} {windows_disk} 512M 40")
    installer.succeed("sha256sum /dev/vdb1 > /tmp/esp-before")
    windows_before = windows_digest(installer)
    installer.succeed(
        f"{sgdisk} --new=5:$({sgdisk} --first-aligned-in-largest {windows_disk} | tail -n 1):$({sgdisk} --end-of-largest {windows_disk} | tail -n 1) {windows_disk} >&2 && "
        f"{partprobe} {windows_disk} && udevadm settle && printf old | {cryptsetup} luksFormat -q --pbkdf pbkdf2 --key-file=- {windows_disk}5 && "
        f"{sgdisk} --delete=5 {windows_disk} >&2 && {partprobe} {windows_disk} && udevadm settle"
    )
    installer.succeed(f"{install_beside_windows} {windows_disk} {installed_system} >&2")
    assert windows_digest(installer) == windows_before, "the install changed a Windows partition"
    installer.succeed("findmnt -no SOURCE /mnt/boot | xargs realpath | grep -qx /dev/vdb1")
    installer.succeed("test -e /mnt/boot/EFI/Microsoft/Boot/fonts.bin")
    installer.succeed("mkdir -p /mnt/nix/store && mount --bind /nix/store /mnt/nix/store")
    installer.succeed("mkdir -p /mnt/etc && touch /mnt/etc/NIXOS")
    installer.succeed(f"mkdir -p /mnt/nix/var/nix/profiles && nix-env -p /mnt/nix/var/nix/profiles/system --set {installed_system}")
    installer.succeed(f"NIXOS_INSTALL_BOOTLOADER=1 nixos-enter --root /mnt -- {installed_system}/bin/switch-to-configuration boot")
    installer.succeed("sync")
    installer.shutdown()

machine = create_test_machine(oldmachine=installer, name="beside_machine")
machine.start()
unlock_disk(machine)
machine.wait_for_unit("multi-user.target")
windows_disk = "/dev/vda"
assert windows_digest() == windows_before, "the first boot changed a Windows partition"
machine.succeed("findmnt -no SOURCE /boot | xargs realpath | grep -qx /dev/vda1")
machine.succeed("test ! -e /boot/EFI/Microsoft/Boot/copied-by-nixos")
machine.succeed("sbctl sign /boot/EFI/Microsoft/Boot/bootmgfw.efi >&2")
machine.shutdown()

machine.start()
unlock_disk(machine)
machine.wait_for_unit("multi-user.target")
