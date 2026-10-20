def record_boot_time():
    import json

    boot_time, problem = measure_boot_time(machine)
    if boot_time is None:
        machine.log(f"boot time not recorded: {problem}")
        return

    with open(machine.out_dir / "boot-time.json", "w") as file:
        json.dump(boot_time, file)


record_boot_time()
