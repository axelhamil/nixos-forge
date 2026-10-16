{
  configurations.nixos.example = {
    stateVersion = "26.05";
    disk = "/dev/nvme0n1";
    hardware = [
      "common-pc-ssd"
      "common-cpu-intel-cpu-only"
    ];

    profile = "desktop";
    features = [
      "secure-boot"
      "plymouth"
      "hyprland"
    ];
    apps = [ "firefox" ];

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
