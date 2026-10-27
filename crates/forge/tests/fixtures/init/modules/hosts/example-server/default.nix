{
  configurations.nixos.example-server = {
    system = "x86_64-linux";
    profile = "server";

    features = [
      "disk" # LUKS and btrfs, `/` wiped at every boot
      "auto-upgrade" # daily, the version CI validated
    ];

    flake = "github:alice/nixos";
    # mirrors = [ "https://codeberg.org/alice/nixos.git" ]; # other copies of your fork
    # signers = [ "ssh-ed25519 AAAA... deploy" ]; # keys signing deploy/<host>; init writes yours

    users.alice = {
      # admin = false; # true by default: may run sudo
    };
  };
}
