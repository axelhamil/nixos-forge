from subprocess import run as run_on_host


def create_secure_machine(name, state, installer, *args):
    run_on_host([prepare_secure_machine, state], check=True)
    secure = create_machine(
        start_command=machine_command(
            secure_machine,
            *disks(installer, 1),
            f"-drive if=pflash,format=raw,unit=1,file={state}/vars.fd",
            f"-chardev socket,id=chrtpm,path={state}/swtpm/socket.ctrl",
            *args,
        ),
        name=name,
    )
    driver.machines_qemu.append(secure)
    return secure
