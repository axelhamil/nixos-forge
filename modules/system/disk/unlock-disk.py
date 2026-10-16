test_passphrase = "secretsecret"


def unlock_disk(machine):
    machine.wait_for_console_text(unlock_prompt)
    machine.send_chars(f"{test_passphrase}\n")

