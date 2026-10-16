{
  configurations.nixos.example = {
    stateVersion = "26.05";
    hardware = [
      "common-pc-ssd"
      "common-cpu-intel-cpu-only"
    ];

    profile = "desktop";
    features = [
      "disk"
      "secure-boot"
      "plymouth"
      "hyprland"
      "docker"
      "direnv"
      "nix-ld"
      "libvirt"
      "ssh"
    ];
    apps = [ "firefox" ];
    flake = "/home/alice/nixos";

    users.alice = {
      fullName = "Alice Example";
      email = "alice@example.org";
    };

    module.virtualisation.vmVariant.virtualisation = {
      memorySize = 8192;
      cores = 4;
    };
  };
}
