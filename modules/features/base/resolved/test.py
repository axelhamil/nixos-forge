machine.wait_for_unit("systemd-resolved.service")
machine.succeed("grep -q '^nameserver 127.0.0.53$' /etc/resolv.conf")
machine.succeed("resolvectl query machine")
status = machine.succeed("resolvectl status")
assert "Fallback DNS" not in status, "resolved falls back to DNS servers compiled into systemd"
assert "-LLMNR" in status, "LLMNR is on"
