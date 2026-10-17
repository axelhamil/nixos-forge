from json import loads as parse_json
import re

luks = "/dev/disk/by-partlabel/disk-main-luks"
boot_count_path = "/sys/firmware/efi/efivars/LoaderBootCountPath-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"


def boot_entries():
    entries = {}
    for entry in parse_json(machine.succeed("bootctl list --json=short --no-pager")):
        generation = re.search(r"Generation (\d+)", entry.get("version") or "")
        if not generation:
            continue
        counter = re.search(r"(\+\d+(?:-\d+)?)\.(?:conf|efi)$", entry["path"])
        entries[int(generation[1])] = counter[1] if counter else ""
    return entries


def current_system():
    return machine.succeed("readlink -f /run/current-system").strip()


def wait_until_assessed():
    machine.wait_until_succeeds(f"! test -e {boot_count_path} || systemctl is-active systemd-bless-boot.service")


def boot():
    if machine.booted:
        machine.reboot()
    else:
        machine.start(allow_reboot=True)
    if not unlocks_with_tpm:
        unlock_disk(machine)
    machine.wait_for_unit("multi-user.target")


unlocks_with_tpm = machine.execute(f"{cryptsetup} luksDump --dump-json-metadata {luks} | grep -q systemd-tpm2")[0] == 0
machine.shutdown()
installer = copy_store_to_disk()

machine = create_machine(start_command=" ".join(upgraded_machine_command(installer)), name="upgraded_machine")
driver.machines_qemu.append(machine)

with subtest("the generation that started is blessed"):
    boot()
    machine.fail("findmnt -t 9p /nix/store")
    wait_until_assessed()
    print(machine.succeed("bootctl list --no-pager"))
    previous = current_system()
    assert boot_entries() == {1: ""}, f"generation 1 still has a counter: {boot_entries()}"

with subtest("a generation installed for the next boot gets 3 tries"):
    broken = machine.succeed("readlink -f /etc/rollback-test/next-generation").strip()
    machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {broken}")
    machine.succeed(f"{broken}/bin/switch-to-configuration boot")
    assert boot_entries()[2] == "+3", f"generation 2 did not get 3 tries: {boot_entries()}"

for tries_done in range(1, 4):
    with subtest(f"try {tries_done} of the generation that does not start is not blessed"):
        boot()
        machine.succeed(f"test -e {boot_count_path}")
        assert current_system() == broken, f"try {tries_done} booted {current_system()}, not {broken}"
        machine.wait_until_succeeds("systemctl is-failed systemd-boot-check-no-failures.service")
        machine.fail("systemctl is-active systemd-bless-boot.service")
        counter = f"+{3 - tries_done}-{tries_done}"
        assert boot_entries()[2] == counter, f"expected generation 2 at {counter}: {boot_entries()}"

with subtest("once its tries are spent, the machine starts the previous generation"):
    boot()
    print(machine.succeed("bootctl list --no-pager"))
    assert current_system() == previous, f"booted {current_system()}, not the previous generation {previous}"
    wait_until_assessed()
    assert boot_entries() == {1: "", 2: "+0-3"}, f"generation 2 is not marked bad: {boot_entries()}"
