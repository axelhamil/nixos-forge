import json
import statistics

settle_seconds = 10
metrics = ("total", "userspace", "memory", "services")


def measure(host, side, round_number):
    vm = create_machine(start_command=hosts[host][side]["vm"], name=f"{host}_{side}_{round_number}")
    driver.machines_qemu.append(vm)
    vm.start()
    boot_time, problem = measure_boot_time(vm, wait=300)
    if boot_time is None:
        raise RuntimeError(f"{host} ({side}, round {round_number}) could not be measured: {problem}")

    vm.succeed(f"sleep {settle_seconds}")
    memory = {
        key: int(value.split()[0])
        for key, value in (line.split(":", 1) for line in vm.succeed("cat /proc/meminfo").splitlines())
    }
    services = vm.succeed("systemctl list-units --type=service --state=running --no-legend --plain | wc -l")
    sample = boot_time | {
        "memory": round((memory["MemTotal"] - memory["MemAvailable"]) / 1024, 1),
        "services": int(services.strip()),
    }
    vm.log(f"{host} ({side}, round {round_number}): {sample}")
    vm.shutdown()
    return sample


samples = {host: {"base": [], "current": []} for host in hosts}
for round_number in range(1, rounds + 1):
    for host in sorted(hosts):
        for side in ("base", "current"):
            with subtest(f"{host}: {side} generation, boot {round_number} of {rounds}"):
                samples[host][side].append(measure(host, side, round_number))


def summary(host, side):
    with open(hosts[host][side]["closure"]) as file:
        closure = int(file.read().strip())
    boots = samples[host][side]
    return {
        "min": {key: min(boot[key] for boot in boots) for key in metrics},
        "median": {key: statistics.median(boot[key] for boot in boots) for key in metrics},
        "closure": closure,
        "samples": boots,
    }


report = {
    "rounds": rounds,
    "skipped": skipped,
    "hosts": {host: {side: summary(host, side) for side in ("base", "current")} for host in sorted(hosts)},
}
with open(driver.out_dir / "perf.json", "w") as file:
    json.dump(report, file, indent=2)
print(json.dumps(report, indent=2))
