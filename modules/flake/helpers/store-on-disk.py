def copy_store_to_disk():
    installer = next(m for m in driver.machines_qemu if m.name == "machine")
    installer.start()
    installer.wait_for_unit("multi-user.target")
    installer.succeed(f"printf %s {test_passphrase} | {cryptsetup} open --key-file=- /dev/disk/by-partlabel/disk-main-luks target")
    installer.succeed("mkdir -p /target/nix && mount -o subvol=@nix /dev/mapper/target /target/nix")
    installed_system = installer.succeed("readlink /target/nix/var/nix/profiles/system-1-link").strip()
    installer.succeed(f"nix --extra-experimental-features nix-command copy --no-check-sigs --to /target {installed_system} >&2", timeout=timedelta(minutes=10))
    installer.succeed(f"umount /target/nix && {cryptsetup} close target")
    installer.shutdown()
    return installer
