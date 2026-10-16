start_all()
server.wait_for_unit("sshd.service")
server.succeed("test -s /persist/etc/ssh/ssh_host_ed25519_key")
server.fail("ls /persist/etc/ssh /etc/ssh | grep -q rsa")

client.succeed("mkdir -m 700 /root/.ssh && ssh-keygen -q -t ed25519 -N '' -f /root/.ssh/id_ed25519")
public_key = client.succeed("cat /root/.ssh/id_ed25519.pub")
server.succeed(f"install -D -m 600 -o tester /dev/stdin /home/tester/.ssh/authorized_keys <<< '{public_key}'")
server.succeed(f"install -D -m 600 /dev/stdin /root/.ssh/authorized_keys <<< '{public_key}'")

ssh = "ssh -o StrictHostKeyChecking=accept-new -o BatchMode=yes"
handshake = client.succeed(f"{ssh} -v tester@server true 2>&1")
assert "kex: algorithm: mlkem768x25519-sha256" in handshake, "key exchange is not post-quantum"
client.fail(f"{ssh} -o KexAlgorithms=diffie-hellman-group-exchange-sha256 tester@server true")

refused = client.fail(f"{ssh} -o PubkeyAuthentication=no tester@server true 2>&1")
assert "Permission denied (publickey)" in refused, "sshd offers more than public keys"
client.fail(f"{ssh} root@server true")
