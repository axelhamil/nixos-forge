machine.wait_for_unit("multi-user.target")
machine.succeed("systemctl is-active nh-clean.timer")
machine.fail("systemctl cat nix-gc.timer")
