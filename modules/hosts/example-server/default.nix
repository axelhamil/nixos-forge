{
  configurations.nixos.example-server = {
    stateVersion = "26.05";
    disk = "/dev/sda";

    profile = "server";
    features = [
      "wipe-home"
      "ssh"
    ];

    users.alice = {
      fullName = "Alice Example";
      sshKeys = [ ];
    };

    testScript = ''
      machine.wait_for_open_port(22)
    '';
  };
}
