def upgraded_machine_command(installer):
    subprocess.run([prepare_secure_machine, state], check=True)
    return [
        secure_machine,
        f"-drive if=pflash,format=raw,unit=1,file={state}/vars.fd",
        f"-chardev socket,id=chrtpm,path={state}/swtpm/socket.ctrl",
        *disks(installer, 1),
    ]
