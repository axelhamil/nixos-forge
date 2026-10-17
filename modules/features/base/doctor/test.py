import re


def doctor(command="doctor"):
    status, output = machine.execute(f"{command} 2>&1")
    print(output)
    return status, output


def line(status, check, output):
    return re.search(rf"\[\s*{status}\]\s+{check}\s", output) is not None


machine.wait_for_unit("multi-user.target")
machine.succeed(
    "git init --quiet --bare -b main /srv/origin.git",
    "git clone --quiet /srv/origin.git /srv/nixos",
    "git -C /srv/nixos -c user.name=t -c user.email=t@example.org commit --quiet --allow-empty -m first",
    "git -C /srv/nixos push --quiet origin main",
    "git clone --quiet /srv/origin.git /srv/elsewhere",
    "git -C /srv/elsewhere -c user.name=t -c user.email=t@example.org commit --quiet --allow-empty -m second",
    "git -C /srv/elsewhere push --quiet origin main",
    "chown -R alice /srv/nixos /srv/origin.git",
)

status, output = doctor()
assert status == 0, "doctor failed on a healthy machine"
for check in ["Secure Boot", "LUKS unlock", "Snapshots", "Impermanence"]:
    assert line("SKIP", check, output), f"{check} is not skipped on a machine without it"
assert line("OK", "Disk space", output)
assert line("OK", "Failed units", output)
assert line("WARN", "Updates", output) and "behind by 1" in output, "doctor missed a checkout behind its upstream"
assert machine.succeed("stat -c %U /srv/nixos/.git/FETCH_HEAD").strip() == "alice", "doctor fetched as root"

machine.succeed("touch /srv/nixos/dirty && chown alice /srv/nixos/dirty")
status, output = doctor()
assert "uncommitted changes" in output, "doctor missed a dirty checkout"

machine.succeed("systemd-run --unit=doctor-broken false")
machine.wait_until_succeeds("systemctl is-failed doctor-broken.service")
status, output = doctor()
assert status == 1, "doctor exits 0 with a failed unit"
assert line("FAIL", "Failed units", output) and "doctor-broken.service" in output

status, output = doctor("runuser -u alice -- doctor")
assert "Run sudo doctor" in output, "doctor does not say it runs without root"

status, output = doctor()
assert line("SKIP", "Upgrades", output) and "auto-upgrade is off" in output, "doctor checks upgrades without auto-upgrade"
assert line("SKIP", "Boot", output), "doctor reads boot entries without systemd-boot"

upgrades_doctor = "/run/current-system/specialisation/upgrades/sw/bin/doctor"


def seed_state(outcome, age, message, changes=""):
    machine.succeed(
        "mkdir -p /var/lib/auto-upgrade",
        f"jq -n --arg time \"$(date -d '{age}' --iso-8601=seconds)\" --arg outcome {outcome}"
        f" --arg message '{message}' --arg changes '{changes}'"
        " '{ $time, $outcome, from: \"a\", to: \"b\", source: \"s\", $message, $changes }'"
        " > /var/lib/auto-upgrade/state.json",
    )


status, output = doctor(upgrades_doctor)
assert line("SKIP", "Upgrades", output) and "never ran yet" in output, "doctor misses an auto-upgrade that never ran"

seed_state("refused", "1 hour ago", "this machine runs uncommitted changes")
status, output = doctor(upgrades_doctor)
assert line("WARN", "Upgrades", output), "doctor does not warn about a refused upgrade"
assert "this machine runs uncommitted changes" in output, "doctor hides why the upgrade was refused"

seed_state("rolled-back", "1 hour ago", "b did not start")
status, output = doctor(upgrades_doctor)
assert line("WARN", "Upgrades", output) and "b did not start" in output, "doctor does not warn about a rollback"

seed_state("upgraded", "yesterday", "applies at next boot", "Closure size: 10 -> 12 (3 paths added)")
status, output = doctor(upgrades_doctor)
assert line("OK", "Upgrades", output), "doctor does not report a recent upgrade"
assert "Closure size: 10 -> 12" in output, "doctor does not say what the upgrade changed"

seed_state("upgraded", "20 days ago", "applies at next boot")
status, output = doctor(upgrades_doctor)
assert line("WARN", "Upgrades", output), "doctor misses an upgrade older than 14 days"

machine.succeed(
    "printf '%s\\t%s\\t%s\\t%s\\t%s\\n'"
    " 2026-01-01T04:00:00+00:00 waiting '' '' ''"
    " 2026-01-02T04:00:00+00:00 upgraded aaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbb https://example.org/fork.git"
    " 2026-01-03T04:00:00+00:00 up-to-date bbbbbbbbbbbbbbbbbbbb bbbbbbbbbbbbbbbbbbbb https://example.org/fork.git"
    " > /var/lib/auto-upgrade/history.tsv"
)
history = machine.succeed(f"{upgrades_doctor} --history")
print(history)
rows = [row for row in history.splitlines() if row.startswith("2026-01-0")]
assert [row.split()[1] for row in rows] == ["waiting", "upgraded", "up-to-date"], "doctor --history does not list the runs"

seed_state("up-to-date", "1 hour ago", "deploy/machine is the running commit")
status, output = doctor(upgrades_doctor)
assert line("OK", "Upgrades", output), "doctor does not report an up to date machine"
assert "up to date since 2026-01-02 04:00" in output, "doctor does not date the upgrade the machine is up to date with"

machine.succeed(
    "printf '%s\\t%s\\t%s\\t%s\\t%s\\n'"
    " 2026-10-01T04:00:00+00:00 upgraded aaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbb https://example.org/fork.git"
    " 2026-10-02T04:00:00+00:00 unreachable bbbbbbbbbbbbbbbbbbbb '' ''"
    " 2026-10-17T04:00:00+00:00 up-to-date bbbbbbbbbbbbbbbbbbbb bbbbbbbbbbbbbbbbbbbb https://example.org/fork.git"
    " > /var/lib/auto-upgrade/history.tsv"
)
status, output = doctor(upgrades_doctor)
assert "up to date since 2026-10-01 04:00" in output, "an unreachable night moves the date the machine is up to date since"

for broken in ['{"time":"x"', '{"time":"not-a-date","outcome":"upgraded"}']:
    machine.succeed(f"echo '{broken}' > /var/lib/auto-upgrade/state.json")
    status, output = doctor(upgrades_doctor)
    assert line("WARN", "Upgrades", output) and "cannot read /var/lib/auto-upgrade/state.json" in output, f"doctor misreads {broken}"
    assert line("SKIP", "Boot", output) and "\nfailed: " in output, f"{broken} stops doctor before its other checks"
