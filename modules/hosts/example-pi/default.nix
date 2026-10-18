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
      # "greetd" # the tuigreet login screen, which starts Hyprland by default when it is there (docs/desktop.md)
      # "docker" # rootless Docker (docs/dev.md)
      # "libvirt" # virtual machines with virt-manager (docs/dev.md)
      # "nix-ld" # run binaries not built for NixOS, and AppImages (docs/dev.md)
      "ssh" # key-only SSH server (docs/services.md)
      # "printing" # CUPS with driverless network printers found through Avahi (docs/services.md)
      # "secrets" # sops-nix secrets (docs/services.md)
      # "auto-upgrade" # daily, the version CI validated for this machine: applies at next boot, no automatic rollback on a Pi (docs/raspberry-pi.md)
    ];

    # Hardware: a module of the starter, or any nixos-hardware profile by name
    # (https://github.com/NixOS/nixos-hardware).
    hardware = [
      # "raspberry-pi-4" # Raspberry Pi 4 (docs/raspberry-pi.md)
      "raspberry-pi-5" # Raspberry Pi 5 (docs/raspberry-pi.md)
      # "firmware" # redistributable firmware, CPU microcode and fwupd
      # "audio" # PipeWire with ALSA and PulseAudio support, and realtime scheduling
      # "bluetooth" # Bluetooth, powered on at boot, paired devices kept
      # "nvidia" # NVIDIA GPU with the open kernel module, x86_64-linux only
      # "amd-gpu" # AMD GPU
      # "intel-gpu" # Intel GPU with video decoding
    ];

    # Hardware report from nixos-facter, saved next to this file (docs/install.md).
    # facter = ./facter.json;

    # Apps for every user: an app of modules/apps/<name> with its config, else any nixpkgs
    # package by name ("htop", "tmux"); unfree ones are allowed. Each app stands alone and
    # brings no other: list every one you want. Shell tools usually go in the person's apps.
    apps = [
      # "hyprland" # the Hyprland window manager, with uwsm, its portal and a polkit agent (docs/desktop.md)
      # "waybar" # a status bar (docs/desktop.md)
      # "fuzzel" # an app launcher on Super+D, and the clipboard history on Super+C (docs/desktop.md)
      # "kitty" # a terminal, the default one of Super+Return (docs/desktop.md)
      # "mako" # a notification daemon (docs/desktop.md)
      # "lock" # hyprlock and hypridle: a lock screen on Super+L and after 5 minutes idle (docs/desktop.md)
      # "upgrade-notify" # a desktop notification after each auto-upgrade (docs/updates.md)
      # "direnv" # per-project dev shells with nix-direnv (docs/dev.md)
      # "firefox" # Firefox with telemetry off, profile kept (docs/desktop.md)
      # "zsh" # zsh, the login shell of whoever has it (docs/apps.md)
      # "starship" # the starship prompt (docs/apps.md)
      # "git" # git with sensible defaults (docs/apps.md)
      # "delta" # delta, a pager for git diffs (docs/apps.md)
      # "neovim" # neovim as $EDITOR (docs/apps.md)
      # "fzf" # fuzzy finder on Ctrl+T, Ctrl+R, Alt+C (docs/apps.md)
      # "zoxide" # `z` jumps to a directory you visited (docs/apps.md)
      # "eza" # file listings with git status, as ll, la, lt (docs/apps.md)
      # "bat" # `cat` with syntax highlighting (docs/apps.md)
      # "btop" # a resource monitor (docs/apps.md)
      # "stow" # links your dotfiles repository into your home (docs/apps.md)
    ];

    # Where the machine updates from (docs/updates.md): a flake reference such as
    # "github:<you>/<fork>", whose deploy/<host> branch auto-upgrade pulls once CI passed.
    # flake = "github:alice/nixos";
    # mirrors = [ "https://codeberg.org/alice/nixos.git" ]; # other copies of your fork, tried in order when the first is unreachable
    # signers = [ "ssh-ed25519 AAAA... deploy" ]; # keys signing deploy/<host>: auto-upgrade applies only what they signed, `nix run .#init` writes yours

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
