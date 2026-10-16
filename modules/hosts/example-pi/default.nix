{
  configurations.nixos.example-pi = {
    system = "aarch64-linux";
    stateVersion = "26.05";

    hardware = [ "raspberry-pi-5" ];

    profile = "pi";
    features = [ "ssh" ];

    users.alice = {
      fullName = "Alice Example";
      sshKeys = [ ];
    };
  };
}
