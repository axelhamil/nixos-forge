import shlex

monitor_log = "/home/tester/notify.log"
seen_file = "/home/tester/.local/state/upgrade-notify/seen"


def tester_command(command):
    return f"su - tester -c {shlex.quote('export XDG_RUNTIME_DIR=/run/user/$(id -u); ' + command)}"


def as_tester(command):
    return machine.succeed(tester_command(command))


def seed_state(outcome, time, to, changes=""):
    machine.succeed(
        "mkdir -p /var/lib/auto-upgrade",
        f"jq -n --arg outcome {outcome} --arg time {time} --arg to {to} --arg changes '{changes}'"
        " '{ $time, $outcome, from: \"a\", $to, source: \"s\", message: \"applies at next boot\", $changes }'"
        " > /var/lib/auto-upgrade/state.json",
    )


def monitored():
    return machine.succeed(f"cat {monitor_log}")


def notifications_after(probe):
    as_tester(f"notify-send {probe} probe")
    machine.wait_until_succeeds(f"grep -q '\"{probe}\"' {monitor_log}", timeout=60)
    return monitored().count('"System updated"')


def run_notifier():
    machine.succeed("systemctl --user -M tester@ start upgrade-notify.service")


def start_session(probe):
    machine.wait_for_unit("multi-user.target")
    machine.wait_until_succeeds("systemctl --user -M tester@ is-active default.target")
    machine.succeed("systemd-run --user -M tester@ --unit=notification-stub /run/current-system/sw/bin/notification-stub")
    machine.wait_until_succeeds(tester_command("busctl --user status org.freedesktop.Notifications"), timeout=60)
    machine.succeed(
        "systemd-run --user -M tester@ --unit=notify-monitor /bin/sh -c"
        f" 'exec /run/current-system/sw/bin/dbus-monitor --session type=method_call,interface=org.freedesktop.Notifications,member=Notify >> {monitor_log}'"
    )
    return notifications_after(probe)


machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger tester")
assert start_session("probe-ready") == 0

wanted_by = machine.succeed("systemctl --user -M tester@ show -p WantedBy --value upgrade-notify.service")
assert "graphical-session.target" in wanted_by, f"upgrade-notify does not start with the session: {wanted_by}"

seed_state("upgraded", "2026-01-01T04:00:00+00:00", "b", "Closure size: 10 -> 12 (3 paths added)")
run_notifier()
assert notifications_after("probe-first") == 1, "no notification after an upgrade"
assert "Closure size: 10 -> 12" in monitored(), "the notification does not say what changed"
assert monitored().count("Restart to use it") == 1, "the notification does not say the upgrade waits for a restart"

run_notifier()
assert notifications_after("probe-second") == 1, "the same upgrade is notified twice in a session"

machine.shutdown()
machine.start()
assert start_session("probe-after-reboot") == 1
machine.succeed(f"test -s {seen_file}", "test -s /var/lib/auto-upgrade/state.json")
run_notifier()
assert notifications_after("probe-second-session") == 1, "the same upgrade is notified again after the reboot"

seed_state("upgraded", "2026-01-02T04:00:00+00:00", "test", "Closure size: 12 -> 13 (1 paths added)")
run_notifier()
assert notifications_after("probe-running") == 2, "a new upgrade is not notified"
assert "Closure size: 12 -> 13" in monitored(), "the notification does not say what the new upgrade changed"
assert monitored().count("Restart to use it") == 1, "a running upgrade asks for a restart"

machine.succeed(f"rm {seen_file}")
seed_state("up-to-date", "2026-01-03T04:00:00+00:00", "test")
run_notifier()
assert notifications_after("probe-up-to-date") == 2, "a run with nothing upgraded is notified"
