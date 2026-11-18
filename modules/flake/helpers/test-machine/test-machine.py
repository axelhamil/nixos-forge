from pathlib import Path

no_network = ["-nic none"]
shared_store = "-virtfs local,path=/nix/store,security_model=none,mount_tag=nix-store"


def disk_drive(path, index, boot_first=False):
    return [
        f"-drive file={path},id=drive{index},if=none,index={index},format=qcow2,werror=report",
        f"-device virtio-blk-pci,drive=drive{index}" + (",bootindex=1" if boot_first else ""),
    ]


def machine_command(binary, *args):
    return " ".join([binary, *args, *no_network])


def create_uefi_machine(name, state, *args):
    variables = Path(state, "vars.fd")
    if not variables.exists():
        variables.write_bytes(Path(firmware_vars).read_bytes())
    uefi_machine = create_machine(
        start_command=machine_command(
            qemu,
            f"-drive if=pflash,format=raw,unit=0,readonly=on,file={firmware}",
            f"-drive if=pflash,format=raw,unit=1,file={variables}",
            *args,
        ),
        name=name,
    )
    driver.machines_qemu.append(uefi_machine)
    return uefi_machine
