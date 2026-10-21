import json
import os
import subprocess
import tempfile

state = tempfile.mkdtemp()
disk = os.path.join(state, "disk.qcow2")

subprocess.run([qemu_img, "create", "-f", "qcow2", disk, "16G"], check=True)


def password_of(user):
    return f"{user}-password"


def boot(name, *extra):
    vm = create_uefi_machine(name, state, "-m 2048", *disk_drive(disk, 1, boot_first=True), *extra)
    vm.start()
    return vm


installer = boot("installer", f"-cdrom {iso}")
installer.wait_for_unit("multi-user.target")

with subtest("the assistant starts on tty1 and lists the blank disk, not the ISO"):
    installer.wait_until_tty_matches("1", f"Install {host_name}")
    installer.wait_until_tty_matches("1", r"1\) /dev/vda .*\n\s*Disk to install on, 1 to 1:")
    installer.fail("install-host --disk /dev/sr0 --yes")

with subtest("without a question, a missing password stops the install before erasing"):
    installer.succeed(f"printf %s {test_passphrase} > /tmp/passphrase")
    installer.fail("install-host --disk /dev/vda --passphrase-file /tmp/passphrase --yes --no-reboot")
    installer.fail("lsblk -nro PARTLABEL /dev/vda | grep -q .")

with subtest("the assistant installs without a question"):
    for user in users:
        installer.succeed(f"printf %s {password_of(user)} > /tmp/{user}-password")
    password_files = " ".join(f"--password-file {user}=/tmp/{user}-password" for user in users)
    installer.succeed(
        f"install-host --disk /dev/vda --passphrase-file /tmp/passphrase {password_files} --yes --no-reboot >&2",
        timeout=timedelta(minutes=15),
    )
    installer.fail(f"test -e {passphrase_file}")

installer.shutdown()

machine = boot("machine")
unlock_disk(machine)
