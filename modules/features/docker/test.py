as_tester = "sudo -u tester XDG_RUNTIME_DIR=/run/user/1000 DOCKER_HOST=unix:///run/user/1000/docker.sock"

machine.wait_for_unit("multi-user.target")
machine.succeed("install -d -o tester -g users -m 700 /home/tester")
machine.succeed("loginctl enable-linger tester")
machine.wait_until_succeeds(f"{as_tester} systemctl --user is-active docker.service")

info = machine.succeed(as_tester + " docker info --format '{{.Driver}} {{.LoggingDriver}} {{.SecurityOptions}}'")
driver, logging, security = info.split(maxsplit=2)
assert driver == "overlayfs", f"images are not in the containerd store with its overlayfs snapshotter: {driver}"
assert "rootless" in security, "the daemon is not rootless"
assert logging == "local", f"container logs use {logging}, which never rotates"

machine.succeed(f"tar c --files-from /dev/null | {as_tester} docker import - scratch")
machine.succeed(f"{as_tester} docker run --rm -v /nix/store:/nix/store -v /run/current-system/sw/bin:/bin scratch /bin/true")
machine.succeed(f"{as_tester} systemctl --user is-active docker-prune.timer")
machine.succeed(f"{as_tester} systemctl --user show -p Persistent docker-prune.timer | grep -qx Persistent=yes")
