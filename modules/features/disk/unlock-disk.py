from datetime import timedelta

test_passphrase = "secretsecret"


def unlock_disk(machine):
    machine.wait_for_console_text(unlock_prompt, timeout=timedelta(minutes=10))
    machine.send_chars(f"{test_passphrase}\n")

