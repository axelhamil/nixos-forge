machine.wait_for_unit("multi-user.target")
machine.succeed("systemctl is-active nh-clean.timer")
machine.fail("systemctl cat nix-gc.timer")
assert "@wheel" in machine.succeed("nix config show trusted-users").split(), "wheel is not trusted"
machine.succeed("systemctl cat nix-optimise.timer")
nh_clean = "$(systemctl show -P ExecStart nh-clean.service | grep -o '/nix/store/[^ ;]*')"
machine.succeed(f"grep -q -- '--keep-since 14d --keep 5 --keep-one' {nh_clean}")

machine.succeed("/run/current-system/specialisation/forked/bin/switch-to-configuration test")
machine.fail("systemctl cat nix-optimise.timer")
machine.succeed(f"grep -q -- 'clean all --keep 3$' {nh_clean}")
