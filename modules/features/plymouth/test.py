from datetime import timedelta

encrypted = "/run/current-system/specialisation/encrypted"

machine.wait_for_unit("multi-user.target", timeout=timedelta(minutes=5))
machine.succeed("systemctl show -p Result plymouth-quit.service | grep -qx Result=success")
machine.succeed("journalctl -b -o cat _PID=1 | grep -q 'Show Plymouth Boot Screen'")

machine.succeed("echo -n secretsecret | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -")
machine.succeed("echo -n secretsecret | cryptsetup luksOpen -q /dev/vdb cryptroot")
machine.succeed("mkfs.ext4 -q /dev/mapper/cryptroot")
machine.succeed(f"{encrypted}/bin/switch-to-configuration boot")
machine.succeed("sync")
machine.crash()

machine.start()
machine.wait_for_console_text("Please enter passphrase for disk cryptroot", timeout=timedelta(minutes=5))
machine.send_chars("secretsecret\n")
machine.wait_for_unit("multi-user.target", timeout=timedelta(minutes=5))
machine.succeed("findmnt -no SOURCE / | grep -qx /dev/mapper/cryptroot")
machine.succeed("journalctl -b -o cat | grep -q 'Started Forward Password Requests to Plymouth'")
