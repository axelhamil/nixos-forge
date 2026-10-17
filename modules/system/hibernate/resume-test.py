import os
import shutil
import tempfile

machine.shutdown()

installer = next(m for m in driver.machines_qemu if m.name == "machine")
installer.start()
installer.wait_for_unit("multi-user.target")
installer.succeed(f"printf %s {test_passphrase} | {cryptsetup} open --key-file=- /dev/disk/by-partlabel/disk-main-luks target")
installer.succeed("mkdir -p /target/nix && mount -o subvol=@nix /dev/mapper/target /target/nix")
installed_system = installer.succeed("readlink /target/nix/var/nix/profiles/system-1-link").strip()
installer.succeed(f"nix --extra-experimental-features nix-command copy --no-check-sigs --to /target {installed_system} >&2", timeout=1800)
installer.succeed(f"umount /target/nix && {cryptsetup} close target")
installer.shutdown()

state = tempfile.mkdtemp()
variables = os.path.join(state, "vars.fd")
shutil.copyfile(firmware_vars, variables)
os.chmod(variables, 0o644)

machine = create_machine(
    start_command=" ".join([
        qemu,
        "-m 1024",
        *disks(installer, 1),
        f"-drive if=pflash,format=raw,unit=0,readonly=on,file={firmware}",
        f"-drive if=pflash,format=raw,unit=1,file={variables}",
    ]),
    name="hibernating_machine",
)
driver.machines_qemu.append(machine)


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
