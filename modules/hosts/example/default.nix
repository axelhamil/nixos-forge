# One machine, one file. Read it top to bottom: uncomment a line to turn something on,
# comment it out to turn it off. `nix run .#init` writes your own host from this file.
# Every field is described in docs/options.md; anything else goes in `module` below.
{
  configurations.nixos.example = {
    # Platform: "x86_64-linux" for a PC, "aarch64-linux" for a Raspberry Pi (docs/raspberry-pi.md).
    system = "x86_64-linux";

    # NixOS release the machine was first installed with. Set it once, never change it.
    stateVersion = "26.05";

    # Role of the machine (docs/performance.md): "desktop", "laptop" (desktop plus power
    # saving), "server" (headless), "pi" (server tuned for a Raspberry Pi).
    profile = "desktop";

    # What the machine turns on, by name. An unknown name stops the build with the known ones.
    features = [
      "disk" # LUKS and btrfs, `/` wiped at every boot; the disk is picked at install (docs/install.md)
      "secure-boot" # lanzaboote, then the TPM2 unlocks the disk (docs/install.md)
      "wipe-home" # an empty /home at every boot, needs disk (docs/wipe-home.md)
      # "hibernate" # hibernation to an encrypted swap file, for laptops, needs disk (docs/hibernate.md)
      "plymouth" # graphical boot splash and passphrase prompt (docs/desktop.md)
      # "greetd" # the tuigreet login screen alone, for another desktop (docs/desktop.md)
      "docker" # rootless Docker (docs/dev.md)
      # "libvirt" # virtual machines with virt-manager (docs/dev.md)
      "nix-ld" # run binaries not built for NixOS, and AppImages (docs/dev.md)
      "ssh" # key-only SSH server (docs/services.md)
      # "secrets" # sops-nix secrets (docs/services.md)
      "auto-upgrade" # daily, the version CI validated for this machine: applies at next boot, rolls back a generation that does not start (docs/updates.md)
    ];

    # Hardware: a module of the starter, or any nixos-hardware profile by name
    # (https://github.com/NixOS/nixos-hardware), such as "framework-13-7040-amd". Here a
    # desktop PC: Intel Core Ultra 7 265K, GeForce RTX 5070 Ti, two NVMe SSDs.
    hardware = [
      "common-pc-ssd" # nixos-hardware: weekly TRIM
      "common-cpu-intel-cpu-only" # nixos-hardware: Intel microcode, the screens are on the NVIDIA card
      # "firmware" # redistributable firmware, CPU microcode and fwupd, already in desktop and laptop
      # "audio" # PipeWire, already in desktop and laptop
      # "bluetooth" # Bluetooth, already in desktop and laptop
      "nvidia" # NVIDIA GPU with the open kernel module, here a GeForce RTX 5070 Ti
      # "amd-gpu" # AMD GPU
      # "intel-gpu" # Intel GPU with video decoding, for an Intel GPU that drives the screens
      # "raspberry-pi-4" # Raspberry Pi 4, aarch64-linux (docs/raspberry-pi.md)
      # "raspberry-pi-5" # Raspberry Pi 5, aarch64-linux (docs/raspberry-pi.md)
    ];

    # Hardware report saved by the installer in /persist/facter.json: copy it next to this
    # file and uncomment, nixpkgs then loads the drivers that machine needs (docs/install.md).
    # facter = ./facter.json;

    # Apps for every user: an app of modules/apps/<name> with its config, else any nixpkgs
    # package by name ("vlc", "kdePackages.kate"); unfree ones are allowed.
    apps = [
      "hyprland" # Hyprland with greetd, waybar, fuzzel, kitty, mako, hyprlock (docs/desktop.md)
      # "waybar" # the status bar alone, already in hyprland (docs/desktop.md)
      # "fuzzel" # the app launcher alone, already in hyprland (docs/desktop.md)
      # "kitty" # the terminal alone, already in hyprland (docs/desktop.md)
      # "mako" # the notification daemon alone, already in hyprland (docs/desktop.md)
      # "lock" # hyprlock and hypridle alone, already in hyprland (docs/desktop.md)
      # "upgrade-notify" # a desktop notification after each auto-upgrade, already in desktop and laptop (docs/updates.md)
      "direnv" # per-project dev shells with nix-direnv (docs/dev.md)
      "firefox" # Firefox with telemetry off, profile kept (docs/desktop.md)
      # "shell" # zsh, starship, git, neovim, CLI tools; every user has it by default (docs/apps.md)
      # "zsh" # zsh alone as login shell, already in shell (docs/apps.md)
      # "starship" # the starship prompt alone, already in shell (docs/apps.md)
      # "git" # git with delta alone, already in shell (docs/apps.md)
      # "neovim" # neovim as $EDITOR alone, already in shell (docs/apps.md)
      # "cli" # fzf, zoxide, eza, bat, ripgrep, fd, btop, jq alone, already in shell (docs/apps.md)
      # "stow" # links your dotfiles repository into your home (docs/apps.md)
    ];

    # Where the machine updates from (docs/updates.md): your checkout on the machine, which
    # `update` pulls, or a flake reference such as "github:<you>/<fork>". auto-upgrade
    # pulls the deploy/<host> branch of that fork, which CI moves once this host passed.
    flake = "/home/alice/nixos";
    # mirrors = [ "https://codeberg.org/alice/nixos.git" ]; # other copies of your fork, tried in order when the first is unreachable

    timeZone = "Europe/Paris"; # such as "UTC"
    locale = "fr_FR.UTF-8"; # language and formats, such as "en_US.UTF-8"
    keyboard = "fr"; # layout of the console and the desktop, such as "us"

    # Accounts, by login name: each one is a person of modules/users/<name>, who brings
    # name, keys, apps and own config. Only what differs on this machine goes here. The VM
    # logs in with the password "test"; the installer asks for the real one.
    users.alice = {
      # admin = false; # true by default: may run sudo
      # apps = [ "obsidian" ]; # apps for this user on this machine only, added to the person's
      # groups = [ "libvirtd" ]; # extra groups
    };

    # Any other NixOS setting, as plain config. Here: a bigger VM for
    # `nix build .#nixosConfigurations.<host>.config.system.build.vm`.
    module.virtualisation.vmVariant.virtualisation = {
      memorySize = 8192;
      cores = 4;
    };

    # Python lines appended to this host's boot test, run against the `machine` node.
    # testScript = ''
    #   machine.wait_for_open_port(22)
    # '';
  };
}
