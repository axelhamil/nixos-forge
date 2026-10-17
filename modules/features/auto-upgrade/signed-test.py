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
    "git init -q --bare /srv/primary.git",
    "git init -q --bare /srv/mirror.git",
    "install -m 600 /etc/deploy-key /tmp/trusted",
    "ssh-keygen -q -t ed25519 -N '' -C other -f /tmp/unknown",
    "install -d -m 700 /root/.gnupg",
    "gpg --batch --passphrase '' --quick-gen-key 'deploy <deploy@example.org>' ed25519 sign never",
)
a = machine.succeed("git -C /tmp/work rev-parse HEAD~1").strip()
b = machine.succeed("git -C /tmp/work rev-parse HEAD").strip()


def system_profile():
    return machine.execute("readlink -f /nix/var/nix/profiles/system")


system = system_profile()


def promotion(parents, key=None, subject=None, signing_format="ssh"):
    subject = subject or f"deploy machine: checks passed for {parents[-1][:7]}"
    signing = f"-c gpg.format={signing_format} -c user.signingKey={key}" if key else ""
    sign = "-S" if key else ""
    options = " ".join(f"-p {parent}" for parent in parents)
    return machine.succeed(
        f"git -C /tmp/work {signing} commit-tree {sign} {options} -m '{subject}' {parents[-1]}^{{tree}}"
    ).strip()


def publish(repo, commit):
    machine.succeed(f"git -C /tmp/work push -q -f {repo} {commit}:refs/heads/deploy/machine")


def rebuild_calls():
    return machine.succeed("cat /var/log/nixos-rebuild-calls 2>/dev/null || true").splitlines()


def run(current):
    machine.succeed(f"systemctl set-environment CURRENT_REV={current} NEXT_REV={current} NEXT_DID_NOT_START=false")
    machine.succeed("systemctl reset-failed auto-upgrade.service", "systemctl start auto-upgrade.service")
    machine.succeed("systemctl unset-environment CURRENT_REV NEXT_REV NEXT_DID_NOT_START")
    return json.loads(machine.succeed("cat /var/lib/auto-upgrade/state.json"))


def refused(commit, reason):
    publish("/srv/primary.git", commit)
    state = run(a)
    assert state["outcome"] == "refused", state
    assert f"file:///srv/primary.git: {reason}" in state["message"], state
    assert "DEPLOY_SIGNING_KEY" in state["message"], state
    assert rebuild_calls() == [], f"auto-upgrade built a deploy commit that is {reason}"
    assert system_profile() == system, "the system profile moved"


refused(promotion([b]), "unsigned")
refused(promotion([b], "/tmp/unknown"), "signed by a key not in update.signers")
refused(promotion([b], "/tmp/trusted", "deploy other: checks passed for b"), "signed for another host")
refused(promotion([b], "deploy@example.org", signing_format="openpgp"), "not signed with an SSH key")

valid = promotion([b], "/tmp/trusted")
tampered = machine.succeed(
    f"git -C /tmp/work cat-file commit {valid} | sed 's/checks passed/checks skipped/'"
    " | git -C /tmp/work hash-object -t commit -w --stdin"
).strip()
refused(tampered, "signature invalid")

publish("/srv/primary.git", valid)
state = run(a)
assert state["outcome"] == "upgraded", state
assert state["to"] == valid and state["source"] == "file:///srv/primary.git", state
calls = rebuild_calls()
assert len(calls) == 1 and f"rev={valid}" in calls[0], calls

machine.succeed("git -C /tmp/work commit -q --allow-empty -m C")
c = machine.succeed("git -C /tmp/work rev-parse HEAD").strip()
publish("/srv/primary.git", promotion([valid, c]))
following = promotion([valid, c], "/tmp/trusted")
publish("/srv/mirror.git", following)
state = run(valid)
assert state["outcome"] == "upgraded", state
assert state["from"] == valid and state["to"] == following, state
assert state["source"] == "file:///srv/mirror.git", state
assert len(rebuild_calls()) == 2, "auto-upgrade did not build the signed commit of the mirror"

publish("/srv/primary.git", following)
state = run(following)
assert state["outcome"] == "up-to-date", state

publish("/srv/primary.git", valid)
state = run(following)
assert state["outcome"] == "refused" and "moved back" in state["message"], state
assert len(rebuild_calls()) == 2, "auto-upgrade replayed an older signed promotion"

publish("/srv/primary.git", c)
publish("/srv/mirror.git", c)
state = run(following)
assert state["outcome"] == "refused" and "unsigned" in state["message"], state

machine.succeed("git -C /tmp/work commit -q --allow-empty -m D")
d = machine.succeed("git -C /tmp/work rev-parse HEAD").strip()
restarted = promotion([d], "/tmp/trusted")
publish("/srv/primary.git", restarted)
state = run(following)
assert state["outcome"] == "upgraded", state
assert state["from"] == following and state["to"] == restarted, state
assert len(rebuild_calls()) == 3, "a deploy branch reset under a signed host held it back"
applied = machine.succeed("git -C /var/lib/auto-upgrade/repo rev-parse refs/auto-upgrade/next").strip()
assert applied == restarted, f"refs/auto-upgrade/next is {applied}, not the applied {restarted}"
