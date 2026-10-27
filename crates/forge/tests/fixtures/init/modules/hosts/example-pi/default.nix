{
  configurations.nixos.example-pi = {
    system = "aarch64-linux";
    profile = "pi";

    features = [
      "ssh" # key-only SSH server
      # "auto-upgrade" # daily, no automatic rollback on a Pi
    ];

    # flake = "github:alice/nixos";
    # signers = [ "ssh-ed25519 AAAA... deploy" ]; # keys signing deploy/<host>; init writes yours

    users.alice = {
      # admin = false; # true by default: may run sudo
    };
  };
}
