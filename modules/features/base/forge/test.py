machine.wait_for_unit("multi-user.target")

status, output = machine.execute("forge hello a 'b c'")
assert status == 3, f"forge hello exited {status}, not the code of forge-hello"
assert output.strip() == "hello a b c contract=1", output

overview = machine.succeed("forge --help")
assert "hello" in overview and "Prints its arguments" in overview, overview
assert machine.succeed("forge help") == overview

status, output = machine.execute("forge future 2>&1")
assert status == 1, f"forge future exited {status}"
assert "contract 2" in output and "contract 1" in output, output

status, output = machine.execute("forge nowhere 2>&1")
assert status == 1, f"forge nowhere exited {status}"
assert "unknown command `nowhere`" in output, output

machine.succeed("forge check-docs --help")
