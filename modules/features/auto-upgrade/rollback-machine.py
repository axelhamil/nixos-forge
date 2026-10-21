def create_upgraded_machine(installer):
    return create_uefi_machine("upgraded_machine", machine.state_dir, "-m 1024", *disks(installer, 1))
