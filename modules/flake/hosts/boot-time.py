def record_boot_time():
    import json
    import re

    def seconds(span):
        units = {"h": 3600.0, "min": 60.0, "s": 1.0, "ms": 0.001, "us": 0.000001, "μs": 0.000001}
        return round(
            sum(float(value) * units[unit] for value, unit in re.findall(r"(\d+(?:\.\d+)?)(min|ms|us|μs|h|s)\b", span)),
            1,
        )

    state = machine.execute("timeout 120 systemctl is-system-running --wait")[1].strip()
    if state not in ("running", "degraded"):
        machine.log(f"boot time not recorded: the system is {state or 'still starting'}")
        return

    analyze = machine.succeed("systemd-analyze time").splitlines()[0]
    total = re.search(r"= (.+)$", analyze)
    userspace = re.search(r"\+ ([^+=]+) \(userspace\)", analyze)
    if total is None or userspace is None:
        machine.log(f"boot time not recorded: unexpected systemd-analyze output: {analyze}")
        return

    with open(machine.out_dir / "boot-time.json", "w") as file:
        json.dump({"total": seconds(total.group(1)), "userspace": seconds(userspace.group(1))}, file)


record_boot_time()
