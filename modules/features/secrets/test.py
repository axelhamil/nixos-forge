machine.wait_for_unit("multi-user.target")
machine.succeed("grep -qx hello /run/secrets/greeting")
machine.succeed("stat -c %a:%U /run/secrets/greeting | grep -qx 400:root")
machine.succeed("test -s /persist/var/lib/sops-nix/key.txt")
