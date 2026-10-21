machine.shutdown()
installer = copy_store_to_disk()
machine = create_uefi_machine("hibernating_machine", machine.state_dir, "-m 1024", *disks(installer, 1))


def boot_id():
    return machine.succeed("cat /proc/sys/kernel/random/boot_id").strip()


with subtest("the system boots with its store on the disk, so it can hibernate"):
    machine.start()
    unlock_disk(machine)
    machine.wait_for_unit("multi-user.target")
    machine.fail("findmnt -t 9p /nix/store")
    machine.succeed("busctl call org.freedesktop.login1 /org/freedesktop/login1 org.freedesktop.login1.Manager CanHibernate | grep -q yes")

with subtest("hibernation keeps / and the running processes"):
    machine.succeed("touch /hibernate-marker")
    machine.succeed("echo kept-in-memory > /run/hibernate-marker")
    machine.succeed("systemd-run --unit=hibernate-witness sleep infinity")
    witness = machine.succeed("systemctl show -p MainPID --value hibernate-witness").strip()
    hibernated_boot = boot_id()

    machine.execute("systemctl hibernate >&2 &", check_return=False)
    machine.wait_for_shutdown()

    machine.start()
    unlock_disk(machine)
    machine.succeed("test -e /hibernate-marker")
    machine.succeed("grep -qx kept-in-memory /run/hibernate-marker")
    assert boot_id() == hibernated_boot, "the machine booted afresh instead of resuming"
    resumed_witness = machine.succeed("systemctl show -p MainPID --value hibernate-witness").strip()
    assert resumed_witness == witness, f"hibernate-witness is {resumed_witness}, was {witness}"

with subtest("a normal boot after a resume still wipes /"):
    machine.shutdown()
    machine.start()
    unlock_disk(machine)
    machine.wait_for_unit("multi-user.target")
    assert boot_id() != hibernated_boot, "the machine resumed again instead of booting"
    machine.fail("test -e /hibernate-marker")
    machine.fail("test -e /run/hibernate-marker")
