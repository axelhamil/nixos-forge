test_passphrase = "secretsecret"


def unlock_disk(machine):
    machine.wait_for_console_text("password query on /dev/tty1")
    machine.send_chars(f"{test_passphrase}\n")


unlock_disk(machine)
