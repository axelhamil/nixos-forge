# A headless server, read top to bottom: uncomment a line to turn something on, comment it
# out to turn it off. Every field is described in docs/reference/options.md; the commented desktop
# host in modules/hosts/example shows the same fields for a workstation.
{
  configurations.nixos.example-server = {
    # Platform: "x86_64-linux" for a PC, "aarch64-linux" for a Raspberry Pi (docs/how-to/raspberry-pi.md).
    system = "x86_64-linux";

    # NixOS release the machine was first installed with. Set it once, never change it.
    stateVersion = "26.05";

    # Role of the machine (docs/reference/performance.md): "desktop" (NetworkManager, latest kernel,
    # responsive scheduler), "laptop" (desktop with a power-aware scheduler), "server" (headless,
    # tuned) or "pi" (server tuned for a Raspberry Pi).
    profile = "server";

    # What the machine turns on, by name. An unknown name stops the build with the known ones.
    features = [
      "disk" # LUKS and btrfs, `/` wiped at every boot, hourly snapshots; the disk is picked at install (docs/tutorials/install.md)
      # "secure-boot" # lanzaboote with its own keys enrolled at first boot, then the TPM2 unlocks the disk (docs/tutorials/install.md)
      # "fido2" # a security key logs in and runs sudo without a password and, once enrolled, unlocks the disk (docs/how-to/fido2.md)
      "wipe-home" # an empty /home at every boot, needs disk (docs/how-to/wipe-home.md)
      # "hibernate" # hibernation to an encrypted swap file, suspend-then-hibernate on lid close; needs disk (docs/how-to/hibernate.md)
      # "dual-boot" # Windows in the boot menu, Secure Boot kept, its disk read-only, needs disk (docs/how-to/dual-boot.md)
      # "plymouth" # graphical boot splash, quiet boot and the passphrase prompt on it (docs/reference/desktop.md)
      # "greetd" # the tuigreet login screen, which starts Hyprland by default when `hyprland` is listed (docs/reference/desktop.md)
      # "docker" # rootless Docker, with a weekly prune of what is unused (docs/reference/dev.md)
      # "libvirt" # virtual machines with virt-manager, a virtual TPM and USB redirection (docs/reference/dev.md)
      # "nix-ld" # run binaries not built for NixOS, and AppImages (docs/reference/dev.md)
      "ssh" # key-only SSH server, no root login, its host key kept (docs/reference/services.md)
      # "printing" # CUPS with driverless network printers found through Avahi (docs/reference/services.md)
      # "secrets" # sops-nix secrets, decrypted with an age key kept in /persist (docs/reference/services.md)
      "auto-upgrade" # daily, the version CI validated: applied at next boot, rolled back if it does not start (docs/explanation/updates.md)
    ];

    # Hardware: a module of the starter, or any nixos-hardware profile by name
    # (https://github.com/NixOS/nixos-hardware), such as "common-cpu-amd".
    hardware = [
      # "common-pc-ssd" # nixos-hardware: weekly TRIM
      "firmware" # redistributable firmware, CPU microcode and fwupd
      # "audio" # PipeWire with ALSA and PulseAudio support, and realtime scheduling
      # "bluetooth" # Bluetooth, powered on at boot, paired devices kept
      # "nvidia" # NVIDIA GPU with the open kernel module and power management
      # "amd-gpu" # AMD GPU, loaded early in the initrd
      # "intel-gpu" # Intel GPU with video decoding, for an Intel GPU that drives the screens
      # "raspberry-pi-4" # Raspberry Pi 4, aarch64-linux (docs/how-to/raspberry-pi.md)
      # "raspberry-pi-5" # Raspberry Pi 5, aarch64-linux (docs/how-to/raspberry-pi.md)
    ];

    # Hardware report saved by the installer in /persist/facter.json: copy it next to this
    # file and uncomment, nixpkgs then loads the drivers that machine needs (docs/tutorials/install.md).
    # facter = ./facter.json;

    # Apps for every user: an app of modules/apps/<name> with its config, else any nixpkgs
    # package by name ("htop", "tmux"); unfree ones are allowed. Each app stands alone and
    # brings no other: list every one you want. Shell tools usually go in the person's apps.
    apps = [
      # "hyprland" # Hyprland through uwsm, with a polkit agent, a Nerd Font and binds for the apps below (docs/reference/desktop.md)
      # "waybar" # a top bar: workspaces, clock, tray, volume, network, battery (docs/reference/desktop.md)
      # "fuzzel" # an app launcher on Super+D, and the clipboard history on Super+C (docs/reference/desktop.md)
      # "kitty" # a terminal, the default one of Super+Return (docs/reference/desktop.md)
      # "mako" # a notification daemon (docs/reference/desktop.md)
      # "lock" # hyprlock and hypridle: a lock screen on Super+L and after 5 minutes idle (docs/reference/desktop.md)
      # "upgrade-notify" # a desktop notification saying what the last auto-upgrade changed (docs/explanation/updates.md)
      # "direnv" # per-project dev shells with nix-direnv (docs/reference/dev.md)
      # "firefox" # Firefox with telemetry off, profile kept (docs/reference/desktop.md)
      # "zsh" # zsh with suggestions, syntax highlighting and a shared history; the login shell of whoever lists it (docs/reference/apps.md)
      # "starship" # the starship prompt in plain-text symbols (docs/reference/apps.md)
      # "git" # git with the settings git's own developers use; the person's name and email are the author (docs/reference/apps.md)
      # "delta" # delta, a pager for git diffs and blame (docs/reference/apps.md)
      # "neovim" # neovim as $EDITOR, with the compiler, tree-sitter, ripgrep and fd most configs expect (docs/reference/apps.md)
      # "fzf" # fuzzy finder on Ctrl+T, Ctrl+R, Alt+C, listed by fd so .gitignore is respected (docs/reference/apps.md)
      # "zoxide" # `z` jumps to a directory you visited (docs/reference/apps.md)
      # "eza" # file listings with git status, as ll, la, lt (docs/reference/apps.md)
      # "bat" # bat, a `cat` with syntax highlighting (docs/reference/apps.md)
      # "btop" # a resource monitor (docs/reference/apps.md)
      # "stow" # links your dotfiles repository, ~/dotfiles by default, into your home at each switch (docs/reference/apps.md)
      "nix-index" # nix-locate finds the package of a file, and `, <command>` runs a command without installing it (docs/reference/dev.md)
      # "gh" # the GitHub CLI, its login kept with wipe-home (docs/reference/apps.md)
      # "steam" # Steam with Proton-GE and protontricks, its library kept (docs/how-to/specialisations.md)
      # "gamemode" # Feral GameMode, for games started with `gamemoderun` (docs/how-to/specialisations.md)
      # "mangohud" # an FPS and frame time overlay, Shift+F2 logs a run (docs/how-to/specialisations.md)
    ];

    # Variants of the machine, each one this host plus its own lists, chosen at boot or
    # switched to by `forge update -s <name>` (docs/how-to/specialisations.md).
    # specialisations.maintenance.features = [ "nix-ld" ];

    # Where the machine updates from (docs/explanation/updates.md): a flake reference such as
    # "github:<you>/<fork>", whose deploy/<host> branch auto-upgrade pulls once CI passed.
    flake = "github:alice/nixos";
    # mirrors = [ "https://codeberg.org/alice/nixos.git" ]; # other copies of your fork, tried in order when the first is unreachable
    # signers = [ "ssh-ed25519 AAAA... deploy" ]; # keys signing deploy/<host>: auto-upgrade applies only their commits; init writes yours

    # timeZone = "UTC"; # such as "Europe/Paris"
    # locale = "en_US.UTF-8"; # language and formats, such as "fr_FR.UTF-8"
    # keyboard = "us"; # layout of the console, such as "fr"

    # Accounts, by login name: each one is a person of modules/users/<name>, who brings
    # name, SSH keys, apps and own config. Only what differs on this machine goes here. The
    # VM logs in with the password "test"; the installer asks for the real one.
    users.alice = {
      # admin = false; # true by default: may run sudo
      # apps = [ "tmux" ]; # apps for this user on this machine only, added to the person's
      # groups = [ "docker" ]; # extra groups
    };

    # Any other NixOS setting, as plain config.
    # module.services.fail2ban.enable = true;

    # Optional, for CI: Python lines appended to this host's boot test, run against `machine`.
    testScript = ''
      machine.wait_for_open_port(22)
    '';
  };

  # Hosts whose installer ISO CI installs and boots end to end (docs/tutorials/install.md).
  installer.testedHosts = [ "example-server" ];
}
