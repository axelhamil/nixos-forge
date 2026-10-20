import json

start_all()
machine.wait_for_unit("multi-user.target")
machine.succeed(
    "git config --global user.email test@example.org",
    "git config --global user.name test",
    "git config --global init.defaultBranch main",
    "git init -q /tmp/work",
    "git -C /tmp/work commit -q --allow-empty -m A",
    "git -C /tmp/work commit -q --allow-empty -m B",
    "git init -q --bare /srv/mirror.git",
)
a = machine.succeed("git -C /tmp/work rev-parse HEAD~1").strip()
b = machine.succeed("git -C /tmp/work rev-parse HEAD").strip()
waiting = (
    "CI has not validated a version for this host yet; if this lasts, CI has not promoted it:"
    " check that the bot App is set up (docs/how-to/ci.md) and that the host's checks pass"
)


def rebuild_calls():
    return machine.succeed("cat /var/log/nixos-rebuild-calls 2>/dev/null || true").splitlines()


def run(current, following=None, following_started=True):
    machine.succeed(
        f"systemctl set-environment CURRENT_REV={current} NEXT_REV={following or current}"
        f" NEXT_DID_NOT_START={str(not following_started).lower()}"
    )
    machine.succeed("systemctl reset-failed auto-upgrade.service", "systemctl start auto-upgrade.service")
    machine.succeed("systemctl unset-environment CURRENT_REV NEXT_REV NEXT_DID_NOT_START")
    return json.loads(machine.succeed("cat /var/lib/auto-upgrade/state.json"))


state = run(a)
assert state["outcome"] == "waiting", state
assert state["message"] == waiting, state

machine.succeed("git -C /tmp/work push -q /srv/mirror.git main:deploy/machine")

state = run(a)
assert state["outcome"] == "upgraded", state
assert state["source"] == "file:///srv/mirror.git", state
assert state["from"] == a and state["to"] == b, state
assert state["message"] == "applies at next boot", state
assert state["changes"].startswith("Closure size:"), state
calls = rebuild_calls()
assert len(calls) == 1, calls
assert calls[0].startswith("boot "), calls
assert f"rev={b}" in calls[0], calls
assert "--option max-jobs 0 --option always-allow-substitutes true" in calls[0], calls

state = run(a, b)
assert state["outcome"] == "up-to-date", state
assert state["message"] == "deploy/machine is installed and applies at next boot", state
assert len(rebuild_calls()) == 1, "auto-upgrade rebuilt the version installed for next boot"

state = run(b)
assert state["outcome"] == "up-to-date", state
assert len(rebuild_calls()) == 1, "auto-upgrade rebuilt the running commit"

state = run(f"{a}-dirty")
assert state["outcome"] == "refused", state
assert "uncommitted" in state["message"], state

state = run(a, f"{a}-dirty")
assert state["outcome"] == "refused", state
assert "uncommitted" in state["message"], state
assert len(rebuild_calls()) == 1, "auto-upgrade replaced a next boot built from uncommitted changes"

machine.succeed("touch /run/cache-lacks-version")
state = run(a)
assert state["outcome"] == "waiting", state
assert state["message"] == "the cache does not have this version yet", state
assert "local builds are disabled" in machine.succeed("cat /var/lib/auto-upgrade/rebuild.log")
machine.succeed("rm /run/cache-lacks-version")

state = run(a, b, following_started=False)
assert state["outcome"] == "rolled-back", state
short_b = machine.succeed(f"git -C /tmp/work rev-parse --short {b}").strip()
expected = f"{short_b} did not start and the machine went back to the previous generation: fix it and push, CI will validate the fix"
assert state["message"] == expected, state
assert len(rebuild_calls()) == 2, "auto-upgrade reinstalled the version that did not start"

machine.succeed(
    "git -C /tmp/work checkout -q -b fix HEAD~1",
    "git -C /tmp/work commit -q --allow-empty -m D",
    "git -C /tmp/work push -q -f /srv/mirror.git fix:deploy/machine",
)
d = machine.succeed("git -C /tmp/work rev-parse HEAD").strip()
state = run(a, b, following_started=False)
assert state["outcome"] == "upgraded", state
assert state["from"] == a and state["to"] == d, state
assert len(rebuild_calls()) == 3, "auto-upgrade did not install the fix over the version that did not start"



def promote_variants(*names):
    variants = " ".join(f"{name} = {{ }};" for name in names)
    machine.succeed(
        f"echo '{{ outputs = _: {{ nixosConfigurations.machine.config.specialisation = {{ {variants} }}; }}; }}' > /tmp/work/flake.nix",
        "git -C /tmp/work add flake.nix",
        "git -C /tmp/work commit -q -m variants",
        "git -C /tmp/work push -q /srv/mirror.git fix:deploy/machine",
    )


def enter(variant):
    machine.succeed(f"/run/booted-system/specialisation/{variant}/bin/switch-to-configuration test")


enter("variant")
promote_variants("variant", "pinned")
state = run(a)
assert state["outcome"] == "upgraded", state
assert state["message"] == "running now", state
calls = rebuild_calls()
assert len(calls) == 4, calls
assert calls[3].startswith("switch "), calls
assert calls[3].endswith(" --specialisation variant"), "auto-upgrade switched a host running a variant back to its base"

promote_variants("pinned")
state = run(a)
assert state["outcome"] == "upgraded", state
calls = rebuild_calls()
assert len(calls) == 5, calls
assert calls[4].startswith("switch ") and "--specialisation" not in calls[4], "auto-upgrade kept a variant the new version removed"

enter("pinned")
state = run(a)
assert state["outcome"] == "upgraded", state
calls = rebuild_calls()
assert len(calls) == 6, calls
assert calls[5].startswith("boot ") and "--specialisation" not in calls[5], "auto-upgrade passed a variant to nixos-rebuild boot"
machine.succeed("/run/booted-system/bin/switch-to-configuration test")

machine.succeed(
    "git -C /tmp/work checkout -q --orphan elsewhere",
    "git -C /tmp/work commit -q --allow-empty -m C",
    "git -C /tmp/work push -q -f /srv/mirror.git elsewhere:deploy/machine",
)
state = run(b)
assert state["outcome"] == "refused", state
assert "moved back" in state["message"], state
assert len(rebuild_calls()) == 6, "auto-upgrade replaced a system deploy/machine does not contain"

machine.succeed("rm -rf /srv/mirror.git")
state = run(b)
assert state["outcome"] == "unreachable", state
assert len(machine.succeed("cat /var/lib/auto-upgrade/history.tsv").splitlines()) == 14

assert "CURRENT_REV" not in machine.succeed("systemctl cat auto-upgrade.service")
machine.succeed("systemctl is-active auto-upgrade.timer")
assert "04:00" in machine.succeed("systemctl show -p TimersCalendar auto-upgrade.timer")

desktop.wait_for_unit("multi-user.target")
desktop.succeed(
    "su bob -c 'git init -q /home/bob/nixos'",
    "su bob -c 'git -C /home/bob/nixos remote add origin git@github.com:alice/nixos.git'",
)
desktop.succeed("systemctl start auto-upgrade.service")
state = json.loads(desktop.succeed("cat /var/lib/auto-upgrade/state.json"))
assert state["outcome"] == "refused", state
assert "owned by root or an admin" in state["message"], state



def desktop_run():
    desktop.succeed("systemctl reset-failed auto-upgrade.service", "systemctl start auto-upgrade.service")
    return json.loads(desktop.succeed("cat /var/lib/auto-upgrade/state.json"))


desktop.succeed("chown -R alice /home/bob/nixos", "chmod g+w /home/bob/nixos/.git/config")
state = desktop_run()
assert state["outcome"] == "refused", state
assert "owned by root or an admin" in state["message"], "auto-upgrade trusted a group-writable .git/config"

desktop.succeed("chgrp wheel /home/bob/nixos/.git/config")
state = desktop_run()
assert state["outcome"] == "unreachable", "auto-upgrade refused a config writable by wheel only"

desktop.succeed("chgrp users /home/bob/nixos/.git/config", "chmod g-w /home/bob/nixos/.git/config")
state = desktop_run()
assert state["outcome"] == "unreachable", state
desktop.succeed("journalctl -u auto-upgrade.service | grep -F https://github.com/alice/nixos.git")

def set_origin(url):
    desktop.succeed(
        "cp /home/bob/nixos/.git/config /tmp/origin-config",
        f"git config --file /tmp/origin-config remote.origin.url {url}",
        "cat /tmp/origin-config > /home/bob/nixos/.git/config",
    )


for cleartext in ["http://github.com/alice/nixos.git", "git://github.com/alice/nixos.git"]:
    set_origin(cleartext)
    state = desktop_run()
    assert state["outcome"] == "refused", f"auto-upgrade fetched from the cleartext origin {cleartext}: {state}"
    assert "not encrypted" in state["message"], state
set_origin("git@github.com:alice/nixos.git")

desktop.succeed(
    "mv /home/bob/nixos/.git /home/bob/real.git",
    "echo 'gitdir: /home/bob/real.git' > /home/bob/nixos/.git",
    "chown alice /home/bob/nixos/.git",
)
state = desktop_run()
assert state["outcome"] == "refused", state
assert "plain .git folder" in state["message"], "auto-upgrade followed a .git file"

desktop.succeed("rm /home/bob/nixos/.git", "ln -s /home/bob/real.git /home/bob/nixos/.git")
state = desktop_run()
assert state["outcome"] == "refused", state
assert "plain .git folder" in state["message"], "auto-upgrade followed a .git symlink"

desktop.succeed("rm /home/bob/nixos/.git", "git init -q --bare /srv/mirror.git")
state = desktop_run()
assert state["outcome"] == "waiting", "auto-upgrade did not go on with the mirrors for a checkout that has no .git yet"
assert state["message"] == waiting, state
