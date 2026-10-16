machine.wait_for_unit("multi-user.target")

with subtest("the installed system holds the hashed passwords and the hardware report"):
    for user in users:
        machine.succeed(f"grep -q '^\\$y\\$' /persist/passwords/{user}")
    report = json.loads(machine.succeed("cat /persist/facter.json"))
    assert report.get("hardware"), "facter.json has no hardware section"

with subtest("a user logs in with the password typed at install"):
    user = users[0]
    machine.wait_until_tty_matches("1", "login: ")
    machine.send_chars(f"{user}\n")
    machine.wait_until_tty_matches("1", "Password: ")
    machine.send_chars(f"{password_of(user)}\n")
    machine.wait_until_succeeds(f"loginctl list-sessions --no-legend | grep -qw {user}")
