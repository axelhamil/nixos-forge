import json
import subprocess

luks = "/dev/disk/by-partlabel/disk-main-luks"

machine.succeed("test -d /var/lib/sbctl/keys")
machine.succeed("test -e /boot/loader/keys/auto/db.auth")
machine.succeed("systemctl show -p ConditionResult generate-sb-keys | grep -qx ConditionResult=no")
machine.succeed("sbctl verify")
machine.fail("sbctl verify | grep 'is not signed' | grep -v /EFI/nixos/")
machine.shutdown()

installer = next(m for m in driver.machines_qemu if m.name == "machine")
state = machine.state_dir
subprocess.run([prepare_secure_machine, state], check=True)

machine = create_machine(
    start_command=" ".join([
        secure_machine,
        *disks(installer, 1),
        f"-drive if=pflash,format=raw,unit=1,file={state}/vars.fd",
        f"-chardev socket,id=chrtpm,path={state}/swtpm/socket.ctrl",
    ]),
    name="secure_machine",
)
driver.machines_qemu.append(machine)
machine.start(allow_reboot=True)
machine.connected = False
unlock_disk(machine)

machine.wait_for_unit("multi-user.target")
machine.succeed("bootctl status | grep -q 'Secure Boot: enabled (user)'")


def luks_header():
    return json.loads(machine.succeed(f"cryptsetup luksDump --dump-json-metadata {luks}"))


def check_tpm2_enrolled():
    header = luks_header()
    tokens = [t for t in header["tokens"].values() if t["type"] == "systemd-tpm2"]
    assert len(tokens) == 1, f"expected one systemd-tpm2 token, got {header['tokens']}"
    assert len(header["keyslots"]) == 2, f"expected the passphrase and the TPM2 keyslots, got {sorted(header['keyslots'])}"
    assert "31" not in header["keyslots"], "the temporary keyslot 31 is still there"


def journal(unit):
    return machine.succeed(f"journalctl -b -o cat -u {unit}")


machine.wait_for_unit("tpm2-enroll.service")
assert "Kept the volume key" in journal("limit-volume-key"), "the initrd did not keep the volume key for enrollment"
enroll_log = journal("tpm2-enroll")
assert "Enrolled a TPM2 token" in enroll_log, enroll_log
assert "Removed the temporary keyslot 31" in enroll_log, enroll_log
check_tpm2_enrolled()
machine.wait_until_fails("grep -q tpm2-enroll /proc/keys")
machine.succeed(f"printf {test_passphrase} | cryptsetup open --test-passphrase {luks}")
machine.reboot()

machine.wait_for_unit("sysinit.target")
machine.fail("grep -q tpm2-enroll /proc/keys")
machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("tpm2-enroll.service")
assert "dropped its volume key" in journal("limit-volume-key"), "the initrd let the volume key reach stage 2"
assert "No volume key" in journal("tpm2-enroll"), "the volume key reached stage 2"
check_tpm2_enrolled()
assert machine.succeed("systemd-analyze pcrs 15 --json=short").count("0" * 64) == 0, "PCR 15 not extended by the unlock"
