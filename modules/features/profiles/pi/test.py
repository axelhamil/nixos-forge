machine.succeed("grep -q '^ssh://builder@builder.example aarch64-linux' /etc/nix/machines")
assert machine.succeed("nix config show builders").strip() == "@/etc/nix/machines", "remote builders not in use"
assert machine.succeed("nix config show builders-use-substitutes").strip() == "true"
machine.succeed("systemd-analyze cat-config systemd/journald.conf | grep -qx SystemMaxUse=100M")
machine.succeed("systemd-analyze cat-config systemd/system.conf | grep -qx RuntimeWatchdogSec=15s")
machine.succeed("grep -Eq '^%wheel\\s+ALL=\\(ALL:ALL\\)\\s+NOPASSWD' /etc/sudoers")
