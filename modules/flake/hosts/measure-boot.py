def measure_boot_time(machine, wait=120):
    import re

    def seconds(span):
        units = {"h": 3600.0, "min": 60.0, "s": 1.0, "ms": 0.001, "us": 0.000001, "μs": 0.000001}
        return round(
            sum(float(value) * units[unit] for value, unit in re.findall(r"(\d+(?:\.\d+)?)(min|ms|us|μs|h|s)\b", span)),
            1,
        )

    state = machine.execute(f"timeout {wait} systemctl is-system-running --wait")[1].strip()
    if state not in ("running", "degraded"):
        return None, f"the system is {state or 'still starting'}"

    analyze = machine.succeed("systemd-analyze time").splitlines()[0]
    total = re.search(r"= (.+)$", analyze)
    userspace = re.search(r"\+ ([^+=]+) \(userspace\)", analyze)
    if total is None or userspace is None:
        return None, f"unexpected systemd-analyze output: {analyze}"

    return {"total": seconds(total.group(1)), "userspace": seconds(userspace.group(1))}, None
