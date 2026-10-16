{
  configurations.nixos.example-server = {
    stateVersion = "26.05";

    profile = "server";
    features = [
      "disk"
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

  installer.testedHosts = [ "example-server" ];
}
