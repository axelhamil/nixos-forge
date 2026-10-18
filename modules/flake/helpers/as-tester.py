import shlex


def as_tester(command):
    return machine.succeed(f"runuser -l tester -c {shlex.quote(command)}")


machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("home-manager-tester.service")
