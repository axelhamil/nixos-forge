machine.wait_for_unit("multi-user.target")
machine.succeed("touch /home/wipe-home-tester/kept/kept-marker")
machine.succeed(
    "for wiped in /.snapshots/home-wiped.*; do"
    " btrfs property set -ts \"$wiped\" ro false && btrfs subvolume delete --recursive \"$wiped\" || exit 1; done"
)
for stamp in ["20000101T000000", "20000102T000000"]:
    machine.succeed(
        f"btrfs subvolume create /.snapshots/home-wiped.{stamp}"
        f" && btrfs property set -ts /.snapshots/home-wiped.{stamp} ro true"
    )
machine.succeed(
    "su - wipe-home-tester -c 'mkdir -p ~/.ssh ~/.gnupg"
    " && touch ~/.ssh/known_hosts ~/.gnupg/pubring.kbx'"
)
