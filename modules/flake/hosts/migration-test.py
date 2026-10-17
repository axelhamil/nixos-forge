import re

home_state = f"/home/{user}/.ssh/migration-state"


def system_state():
    return {
        "machine-id": machine.succeed("cat /etc/machine-id").strip(),
        "uid": machine.succeed(f"id -u {user}").strip(),
        "persist": machine.succeed("cat /persist/migration-state").strip(),
        "var-lib-nixos": machine.succeed("cat /var/lib/nixos/migration-state").strip(),
    }


with subtest("the base version runs and writes its state"):
    machine.wait_for_unit("multi-user.target")
    machine.succeed(
        "echo persist > /persist/migration-state",
        "echo var-lib-nixos > /var/lib/nixos/migration-state",
        f"runuser -u {user} -- sh -c 'mkdir -p ~/.ssh && echo home > {home_state}'",
    )
    before = system_state()

with subtest("the next version is installed for the next boot"):
    next_system = machine.succeed("readlink -f /etc/migration-test/next").strip()
    generation = 1 + max(
        int(link)
        for link in re.findall(r"system-(\d+)-link", machine.succeed("ls /nix/var/nix/profiles"))
    )
    machine.succeed(
        f"ln -sfn {next_system} /nix/var/nix/profiles/system-{generation}-link",
        f"ln -sfn system-{generation}-link /nix/var/nix/profiles/system",
        f"{next_system}/bin/switch-to-configuration boot",
    )

with subtest("the next version starts and keeps the state"):
    machine.shutdown()
    machine.start()
    unlock_disk(machine)
    machine.wait_for_unit("multi-user.target")
    assert machine.succeed("readlink -f /run/current-system").strip() == next_system, "the machine did not start the next version"
    assert system_state() == before, f"state changed across the switch: {system_state()} instead of {before}"
    if wipes_kept_home:
        machine.succeed(f"grep -qx home /.snapshots/home-wiped.*/{user}/.ssh/migration-state")
    else:
        machine.succeed(f"grep -qx home {home_state}")

with subtest("doctor finds nothing the switch left behind"):
    status, report = machine.execute("doctor --all 2>&1")
    print(report)
    assert re.search(r"\[\s*OK\]\s+Impermanence\s", report), "the next version loses files written since boot"
    failed_line = re.search(r"Failed units\s+(.*)", report)
    assert failed_line, "doctor did not check the failed units"
    failed = failed_line[1]
    units = failed.removeprefix("system: ").split() if failed != "none" else []
    unexpected = set(units) - set(failing_on_shared_store)
    assert not unexpected, f"units failed after the switch: {sorted(unexpected)}"
