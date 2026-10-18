from subprocess import run as run_on_host


def windows_digest(m=None):
    m = m or machine
    contents = [f"sha256sum < {windows_disk}{n}" for n in windows_content]
    entries = [f"{sgdisk} -i {n} {windows_disk} | grep -v '^Partition name'" for n in range(1, 5)]
    return m.succeed(" && ".join(contents + entries))


def wait_for_nixos(m=None):
    m = m or machine
    if not tpm_unlocks:
        unlock_disk(m)
    m.wait_for_unit("multi-user.target")


def start_uefi_machine(name, *drives):
    run_on_host([prepare_secure_machine, state], check=True)
    m = create_machine(
        start_command=" ".join([
            secure_machine,
            "-virtfs local,path=/nix/store,security_model=none,mount_tag=nix-store",
            *disks(installer, 1),
            f"-drive if=pflash,format=raw,unit=1,file={state}/vars.fd",
            f"-chardev socket,id=chrtpm,path={state}/swtpm/socket.ctrl",
            *drives,
        ]),
        name=name,
    )
    driver.machines_qemu.append(m)
    m.start(allow_reboot=True)
    m.connected = False
    wait_for_nixos(m)
    return m


def extra_disk(path):
    run_on_host([qemu_img, "create", "-q", "-f", "qcow2", path, "1G"], check=True)
    return [
        f"-drive file={path},id=windows,if=none,format=qcow2,werror=report",
        "-device virtio-blk-pci,drive=windows",
    ]
