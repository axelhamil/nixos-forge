# A Raspberry Pi, read top to bottom: uncomment a line to turn something on, comment it out
# to turn it off. Every field is described in docs/options.md, the board in
# docs/raspberry-pi.md; the build gives an SD image to flash, not an installer.
{
  configurations.nixos.example-pi = {
    # Platform: "aarch64-linux" for a Raspberry Pi, "x86_64-linux" for a PC.
    system = "aarch64-linux";

    # NixOS release the machine was first installed with. Set it once, never change it.
    stateVersion = "26.05";

    # Role of the machine (docs/performance.md): "pi" is the server profile tuned for a
    # Raspberry Pi; "server", "desktop" and "laptop" also exist.
    profile = "pi";

    # What the machine turns on, by name. An unknown name stops the build with the known ones.
    features = [
      # "disk" # LUKS and btrfs, `/` wiped at every boot, for a PC (docs/install.md)
      # "secure-boot" # lanzaboote, for a PC with UEFI (docs/install.md)
      # "wipe-home" # an empty /home at every boot, needs disk (docs/wipe-home.md)
      # "hibernate" # hibernation to an encrypted swap file, needs disk (docs/hibernate.md)
      # "plymouth" # graphical boot splash (docs/desktop.md)
      # "greetd" # the tuigreet login screen alone, for another desktop (docs/desktop.md)
      # "docker" # rootless Docker (docs/dev.md)
      # "libvirt" # virtual machines with virt-manager (docs/dev.md)
      # "nix-ld" # run binaries not built for NixOS (docs/dev.md)
      "ssh" # key-only SSH server (docs/services.md)
      # "secrets" # sops-nix secrets (docs/services.md)
      # "auto-upgrade" # rebuild from your fork every week, needs a flake reference in `flake` (docs/updates.md)
    ];

    # Hardware: a module of the starter, or any nixos-hardware profile by name
    # (https://github.com/NixOS/nixos-hardware).
    hardware = [
      # "raspberry-pi-4" # Raspberry Pi 4 (docs/raspberry-pi.md)
      "raspberry-pi-5" # Raspberry Pi 5 (docs/raspberry-pi.md)
      # "firmware" # redistributable firmware and fwupd
      # "audio" # PipeWire
      # "bluetooth" # Bluetooth
      # "nvidia" # NVIDIA GPU with the open kernel module, x86_64-linux only
      # "amd-gpu" # AMD GPU
      # "intel-gpu" # Intel GPU with video decoding
    ];

    # Hardware report from nixos-facter, saved next to this file (docs/install.md).
    # facter = ./facter.json;

    # Apps for every user: an app of modules/apps/<name> with its config, else any nixpkgs
    # package by name ("htop", "tmux"); unfree ones are allowed.
    apps = [
      # "hyprland" # Hyprland with greetd, waybar, fuzzel, kitty, mako, hyprlock (docs/desktop.md)
      # "waybar" # the status bar alone, already in hyprland (docs/desktop.md)
      # "fuzzel" # the app launcher alone, already in hyprland (docs/desktop.md)
      # "kitty" # the terminal alone, already in hyprland (docs/desktop.md)
      # "mako" # the notification daemon alone, already in hyprland (docs/desktop.md)
      # "lock" # hyprlock and hypridle alone, already in hyprland (docs/desktop.md)
      # "direnv" # per-project dev shells with nix-direnv (docs/dev.md)
      # "firefox" # Firefox with telemetry off, profile kept (docs/desktop.md)
      # "shell" # zsh, starship, git, neovim, CLI tools; every user has it by default (docs/apps.md)
      # "zsh" # zsh alone as login shell, already in shell (docs/apps.md)
      # "starship" # the starship prompt alone, already in shell (docs/apps.md)
      # "git" # git with delta alone, already in shell (docs/apps.md)
      # "neovim" # neovim as $EDITOR alone, already in shell (docs/apps.md)
      # "cli" # fzf, zoxide, eza, bat, ripgrep, fd, btop, jq alone, already in shell (docs/apps.md)
      # "stow" # links your dotfiles repository into your home (docs/apps.md)
    ];

    # Where the machine updates from (docs/updates.md): a flake reference such as
    # "github:<you>/<fork>" for auto-upgrade, or a checkout on the machine for `update`.
    # flake = "github:alice/nixos";

    # timeZone = "UTC"; # such as "Europe/Paris"
    # locale = "en_US.UTF-8"; # language and formats, such as "fr_FR.UTF-8"
    # keyboard = "us"; # layout of the console, such as "fr"

    # Accounts, by login name: each one is a person of modules/users/<name>. A Pi starts
    # headless and SSH refuses passwords: set `sshKeys` in the person file, they are the
    # only way in (docs/raspberry-pi.md). Only what differs on this machine goes here.
    users.alice = {
      # admin = false; # true by default: may run sudo, without a password on a Pi
      # apps = [ "tmux" ]; # apps for this user on this machine only, added to the person's
      # groups = [ "gpio" ]; # extra groups
    };

    # Any other NixOS setting, as plain config.
    # module.services.fail2ban.enable = true;

    # Python lines appended to this host's boot test; a Pi has none, its image only boots on the board.
    # testScript = "";
  };
}
