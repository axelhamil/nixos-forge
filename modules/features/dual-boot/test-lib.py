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
    m = create_secure_machine(name, state, installer, shared_store, *drives)
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
