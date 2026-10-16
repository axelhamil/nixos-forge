machine.wait_for_unit("multi-user.target")
machine_id = machine.succeed("cat /etc/machine-id")

machine.succeed("touch /root-marker /persist/persist-marker /home/home-marker")
machine.shutdown()

machine.start()
unlock_disk(machine)
machine.wait_for_unit("multi-user.target")

machine.fail("test -e /root-marker")
machine.succeed("test -e /persist/persist-marker")
assert machine.succeed("cat /etc/machine-id") == machine_id, "machine-id changed across reboot"
