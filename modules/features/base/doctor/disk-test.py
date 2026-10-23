import re


def doctor_report():
    _, output = machine.execute("forge doctor --all 2>&1")
    print(output)
    return output


machine.wait_for_unit("multi-user.target")
report = doctor_report()
assert re.search(r"\[\s*OK\]\s+Impermanence\s", report), "a fresh boot already loses files"
assert "/var/lib/doctor-boot-state" not in report, "doctor lists a file tmpfiles writes at boot after the clock stepped back"
assert re.search(r"\[\s*OK\]\s+Snapshots\s", report), "doctor does not see the btrbk snapshots"
assert re.search(r"\[\s*OK\]\s+Boot\s", report), "doctor does not read the boot entries"

machine.succeed(
    "touch /doctor-lost /var/lib/doctor-late /persist/doctor-kept /var/lib/nixos/doctor-kept",
    "mkdir -p /srv/doctor && echo state > /srv/doctor/state",
)
report = doctor_report()
assert re.search(r"\[\s*WARN\]\s+Impermanence\s", report), "doctor does not warn about files lost at boot"
assert "/doctor-lost" in report and "/srv/doctor/state" in report, "doctor misses a file written to /"
assert "/var/lib/doctor-late" in report, "doctor misses a file written under /var after boot"
assert "doctor-kept" not in report, "doctor lists a persisted file"

machine.execute("/run/current-system/bin/switch-to-configuration test")
summary = machine.succeed("forge doctor 2>&1 || true")
print(summary)
assert "3 files written to /" in summary, "a switch leaves files doctor reports as lost"
assert re.search(r"1\s+/srv/doctor\n", summary), "the summary does not group lost files by directory"
