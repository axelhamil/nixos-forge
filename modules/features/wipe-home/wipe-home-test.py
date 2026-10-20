machine.fail("test -e /home/home-marker")
machine.succeed("find /.snapshots -maxdepth 2 -path '/.snapshots/home-wiped.*/home-marker' | grep -q .")
machine.fail("test -e /.snapshots/home-wiped.20000101T000000")
machine.succeed("test -e /.snapshots/home-wiped.20000102T000000")
machine.succeed("for wiped in /.snapshots/home-wiped.*; do btrfs property get -ts \"$wiped\" ro | grep -qx ro=true || exit 1; done")

machine.succeed("test -e /home/wipe-home-tester/kept/kept-marker")
machine.succeed("test -e /persist/home/wipe-home-tester/kept/kept-marker")
owner_and_mode = machine.succeed("stat -c '%U %a' /home/wipe-home-tester").strip()
assert owner_and_mode == "wipe-home-tester 700", f"home directory is {owner_and_mode}"

for kept in [".ssh/known_hosts", ".gnupg/pubring.kbx"]:
    machine.succeed(f"test -e /home/wipe-home-tester/{kept}")
for private in [".ssh", ".gnupg"]:
    owner_and_mode = machine.succeed(f"stat -c '%U %a' /home/wipe-home-tester/{private}").strip()
    assert owner_and_mode == "wipe-home-tester 700", f"~/{private} is {owner_and_mode}"
