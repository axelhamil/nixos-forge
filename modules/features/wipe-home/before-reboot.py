machine.wait_for_unit("multi-user.target")
machine.succeed("touch /home/wipe-home-tester/kept/kept-marker")
machine.succeed("btrfs subvolume create /.snapshots/home-wiped.20000101T000000")
machine.succeed(
    "su - wipe-home-tester -c 'mkdir -p ~/.ssh ~/.gnupg"
    " && touch ~/.ssh/known_hosts ~/.gnupg/pubring.kbx'"
)
