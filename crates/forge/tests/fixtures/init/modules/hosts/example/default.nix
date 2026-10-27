# One machine, one file: `nix run .#init` writes your own host from this file.
{
  configurations.nixos.example = {
    system = "x86_64-linux";

    profile = "desktop";

    features = [
      "disk" # LUKS and btrfs, `/` wiped at every boot
      "secure-boot" # lanzaboote with its own keys
      # "fido2" # a security key logs in and runs sudo
      "wipe-home" # an empty /home at every boot, needs disk
      "plymouth" # graphical boot splash
      "greetd" # the tuigreet login screen
      "docker" # rootless Docker
      # "libvirt" # virtual machines with virt-manager
      "nix-ld" # run binaries not built for NixOS
      "ssh" # key-only SSH server
      "printing" # CUPS with driverless network printers
      "auto-upgrade" # daily, the version CI validated
    ];

    hardware = [
      "common-pc-ssd" # nixos-hardware: weekly TRIM
      "firmware" # redistributable firmware
      "audio" # PipeWire
      "bluetooth" # Bluetooth, powered on at boot
      "nvidia" # NVIDIA GPU with the open kernel module
      # "raspberry-pi-5" # Raspberry Pi 5, aarch64-linux
    ];

    apps = [
      "hyprland" # Hyprland through uwsm
      "waybar" # a top bar
      "fuzzel" # an app launcher
      "kitty" # a terminal
      "mako" # a notification daemon
      "lock" # a lock screen
      "upgrade-notify" # a notification after an auto-upgrade
      "direnv" # per-project dev shells
      "firefox" # Firefox with telemetry off
      # "zsh" # zsh with suggestions
    ];

    flake = "/home/alice/nixos";
    # mirrors = [ "https://codeberg.org/alice/nixos.git" ]; # other copies of your fork
    # signers = [ "ssh-ed25519 AAAA... deploy" ]; # keys signing deploy/<host>; init writes yours

    timeZone = "Europe/Paris"; # such as "UTC"
    locale = "fr_FR.UTF-8"; # such as "en_US.UTF-8"
    keyboard = "fr"; # such as "us"

    users.alice = {
      # admin = false; # true by default: may run sudo
    };
  };
}
