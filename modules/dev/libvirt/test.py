machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("libvirtd.socket")
machine.succeed("virsh -c qemu:///system list")
machine.succeed("btrfs subvolume show /var/lib/libvirt/images")
attributes = machine.succeed("lsattr -d /var/lib/libvirt/images").split()[0]
assert "C" in attributes, "VM images keep copy-on-write"
capabilities = machine.succeed("virsh -c qemu:///system domcapabilities")
assert "<value>emulator</value>" in capabilities, "no emulated TPM for guests"
