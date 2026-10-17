def upgraded_machine_command(installer):
    return [
        qemu,
        "-m 1024",
        f"-drive if=pflash,format=raw,unit=0,readonly=on,file={firmware}",
        f"-drive if=pflash,format=raw,unit=1,readonly=on,file={firmware_vars}",
        *disks(installer, 1),
    ]
