machine.wait_for_unit("systemd-binfmt.service")
machine.succeed("grep -qx enabled /proc/sys/fs/binfmt_misc/appimage_type_2")
machine.succeed("command -v appimage-run")
