encrypted = "/run/current-system/specialisation/encrypted"
relying_party = "pam://nixos-forge"

machine.wait_for_unit("multi-user.target")
machine.succeed("grep -qx 'alice:KeyHandle,UserKey,es256,+presence' /etc/u2f-mappings")
machine.succeed("[ $(wc -l < /etc/u2f-mappings) = 1 ]")

credential = machine.succeed(f"pamu2fcfg -n -o {relying_party} -i {relying_party}").strip()
machine.succeed(f"rm /etc/u2f-mappings && echo 'alice{credential}' > /etc/u2f-mappings")
machine.succeed("su - alice -c 'sudo -S true < /dev/null'")
machine.fail("su - bob -c 'sudo -S true < /dev/null'")
machine.succeed("su - bob -c 'echo password | sudo -S true'")

machine.succeed("echo -n secretsecret | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -")
machine.succeed("echo -n secretsecret | cryptsetup luksOpen -q /dev/vdb cryptroot")
machine.succeed("mkfs.ext4 -q /dev/mapper/cryptroot")
machine.succeed("cryptsetup close cryptroot")
machine.succeed("PASSWORD=secretsecret forge enroll fido2 /dev/vdb")
machine.succeed(f"{encrypted}/bin/switch-to-configuration boot")
machine.succeed("sync")
machine.crash()

machine.start()
machine.wait_for_unit("multi-user.target")
machine.succeed("findmnt -no SOURCE / | grep -qx /dev/mapper/cryptroot")
machine.succeed("journalctl -b -o cat | grep -q 'Unlocked volume via automatically discovered security FIDO2 token'")
