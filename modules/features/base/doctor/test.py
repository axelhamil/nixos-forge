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
