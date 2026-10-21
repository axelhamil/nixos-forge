import json
import re
import statistics

settle_seconds = 10
bench_seconds = 8
bench_threads = 2
bench_directory = "/perf-bench"
metrics = ("total", "userspace", "memory", "services", "cpu", "memoryBandwidth", "randomRead")


def sysbench(vm, test, *options):
    return vm.succeed(
        f"{tools['sysbench']} {test} --threads={bench_threads} --time={bench_seconds} {' '.join(options)} run"
    )


def fio(vm, direct, *options):
    output = vm.succeed(
        f"{tools['fio']} --name=bench --directory={bench_directory} --filename=bench --size=256M"
        f" --direct={direct} --ioengine=libaio --time_based --runtime={bench_seconds}"
        f" --output-format=json {' '.join(options)}"
    )
    return json.loads(output[output.index("{") :])["jobs"][0]


def benchmark(vm):
    cpu = re.search(r"events per second:\s+([\d.]+)", sysbench(vm, "cpu"))
    memory = re.search(
        r"MiB transferred \(([\d.]+) MiB/sec\)",
        sysbench(vm, "memory", "--memory-block-size=1M", "--memory-total-size=1T"),
    )
    if cpu is None or memory is None:
        raise RuntimeError("unexpected sysbench output")

    vm.succeed(f"mkdir -p {bench_directory}")
    direct = 1 if vm.execute(f"dd if=/dev/zero of={bench_directory}/probe bs=4k count=1 oflag=direct")[0] == 0 else 0
    read = fio(vm, direct, "--rw=randread", "--bs=4k", "--iodepth=16")
    vm.succeed(f"rm -rf {bench_directory}")
    return {
        "cpu": round(float(cpu.group(1)), 1),
        "memoryBandwidth": round(float(memory.group(1)), 1),
        "randomRead": round(read["read"]["iops"]),
    }


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
    } | benchmark(vm)
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
