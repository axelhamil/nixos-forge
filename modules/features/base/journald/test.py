machine.succeed("systemd-analyze cat-config systemd/journald.conf | grep -qx SystemMaxUse=500M")
