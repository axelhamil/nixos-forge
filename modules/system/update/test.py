start_all()

desktop.wait_for_unit("multi-user.target")
desktop.succeed(
    "git config --global user.email test@example.org",
    "git config --global user.name test",
    "git config --global init.defaultBranch main",
    "git init --bare /srv/origin.git",
    "git clone /srv/origin.git /srv/nixos",
    "git -C /srv/nixos commit --allow-empty -m first",
    "git -C /srv/nixos push origin main",
    "git clone /srv/origin.git /tmp/elsewhere",
    "git -C /tmp/elsewhere commit --allow-empty -m second",
    "git -C /tmp/elsewhere push origin main",
)
assert desktop.succeed("bash -lc 'echo $NH_FLAKE'").strip() == "/srv/nixos"
desktop.fail("update")
assert desktop.succeed("git -C /srv/nixos log -1 --format=%s").strip() == "second", "update did not pull"

desktop.succeed("git -C /srv/nixos commit --allow-empty -m local")
desktop.succeed("git -C /tmp/elsewhere commit --allow-empty -m third && git -C /tmp/elsewhere push origin main")
desktop.fail("update")
assert desktop.succeed("git -C /srv/nixos log -1 --format=%s").strip() == "local", "update merged a diverged checkout"

server.wait_for_unit("multi-user.target")
server.succeed("systemctl is-active nixos-upgrade.timer")
assert "OnCalendar=Tue 04:00" in server.succeed("systemctl cat nixos-upgrade.timer"), "auto-upgrade is not weekly"
upgrade = server.succeed(
    "systemctl cat nixos-upgrade.service | grep -o '/nix/store/[^ ]*nixos-upgrade-start' | head -1 | xargs cat"
)
assert "--flake github:alice/nixos" in upgrade, "auto-upgrade does not build from the host flake"
desktop.fail("systemctl cat nixos-upgrade.timer")
