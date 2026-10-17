import re


def doctor_report():
    status, output = machine.execute("doctor --all 2>&1")
    print(output)
    return output


machine.wait_for_unit("multi-user.target")
report = doctor_report()
assert re.search(r"\[\s*OK\]\s+Impermanence\s", report), "a fresh boot already loses files"
assert re.search(r"\[\s*OK\]\s+Snapshots\s", report), "doctor does not see the btrbk snapshots"
assert re.search(r"\[\s*OK\]\s+Boot\s", report), "doctor does not read the boot entries"

machine.succeed(
    "touch /doctor-lost /persist/doctor-kept /var/lib/nixos/doctor-kept",
    "mkdir -p /srv/doctor && echo state > /srv/doctor/state",
    "ln -s ../run/doctor-runtime /var/doctor-runtime",
    "ln -s /run /var/doctor-runtime-root",
)
report = doctor_report()
assert re.search(r"\[\s*WARN\]\s+Impermanence\s", report), "doctor does not warn about files lost at boot"
assert "/doctor-lost" in report and "/srv/doctor/state" in report, "doctor misses a file written to /"
assert "doctor-kept" not in report, "doctor lists a persisted file"
assert "doctor-runtime" not in report, "doctor lists a link into /run that tmpfiles recreates at boot"

machine.execute("/run/current-system/bin/switch-to-configuration test")
summary = machine.succeed("doctor 2>&1 || true")
print(summary)
assert "2 files written to /" in summary, "a switch leaves files doctor reports as lost"
assert re.search(r"1\s+/srv/doctor\n", summary), "the summary does not group lost files by directory"
