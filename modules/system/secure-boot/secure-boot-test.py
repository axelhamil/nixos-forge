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

machine.succeed(f"printf {test_passphrase} > /run/passphrase")
machine.succeed(f"systemd-cryptenroll --unlock-key-file=/run/passphrase --tpm2-device=auto --tpm2-pcrs=7+15:sha256={'0' * 64} {luks}")
machine.succeed(f"cryptsetup luksRemoveKey {luks} /run/passphrase")
machine.reboot()

machine.wait_for_unit("multi-user.target")
machine.succeed(f"cryptsetup luksDump {luks} | grep -q systemd-tpm2")
machine.fail(f"printf {test_passphrase} | cryptsetup open --test-passphrase {luks}")
assert machine.succeed("systemd-analyze pcrs 15 --json=short").count("0" * 64) == 0, "PCR 15 not extended by the unlock"
