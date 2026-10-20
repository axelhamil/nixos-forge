# A headless server, read top to bottom: uncomment a line to turn something on, comment it
# out to turn it off. Every field is described in docs/options.md; the commented desktop
# host in modules/hosts/example shows the same fields for a workstation.
{
  configurations.nixos.example-server = {
    # Platform: "x86_64-linux" for a PC, "aarch64-linux" for a Raspberry Pi (docs/raspberry-pi.md).
    system = "x86_64-linux";

    # NixOS release the machine was first installed with. Set it once, never change it.
    stateVersion = "26.05";

    # Role of the machine (docs/performance.md): "desktop" (NetworkManager, latest kernel,
    # responsive scheduler), "laptop" (desktop with a power-aware scheduler), "server" (headless,
    # tuned) or "pi" (server tuned for a Raspberry Pi).
    profile = "server";

    # What the machine turns on, by name. An unknown name stops the build with the known ones.
    features = [
      "disk" # LUKS and btrfs, `/` wiped at every boot, hourly snapshots; the disk is picked at install (docs/install.md)
      # "secure-boot" # lanzaboote with its own keys enrolled at first boot, then the TPM2 unlocks the disk (docs/install.md)
      "wipe-home" # an empty /home at every boot, needs disk (docs/wipe-home.md)
      # "hibernate" # hibernation to an encrypted swap file, and suspend-then-hibernate when the lid closes, needs disk (docs/hibernate.md)
      # "dual-boot" # Windows in the boot menu, Secure Boot kept, its disk read-only, needs disk (docs/dual-boot.md)
      # "plymouth" # graphical boot splash, quiet boot and the passphrase prompt on it (docs/desktop.md)
      # "greetd" # the tuigreet login screen, which starts Hyprland by default when `hyprland` is listed (docs/desktop.md)
      # "docker" # rootless Docker, with a weekly prune of what is unused (docs/dev.md)
      # "libvirt" # virtual machines with virt-manager, a virtual TPM and USB redirection (docs/dev.md)
      # "nix-ld" # run binaries not built for NixOS, and AppImages (docs/dev.md)
      "ssh" # key-only SSH server, no root login, its host key kept (docs/services.md)
      # "printing" # CUPS with driverless network printers found through Avahi (docs/services.md)
      # "secrets" # sops-nix secrets, decrypted with an age key kept in /persist (docs/services.md)
      "auto-upgrade" # daily, the version CI validated for this machine: applies at next boot, rolls back a generation that does not start (docs/updates.md)
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
      # "raspberry-pi-4" # Raspberry Pi 4, aarch64-linux (docs/raspberry-pi.md)
      # "raspberry-pi-5" # Raspberry Pi 5, aarch64-linux (docs/raspberry-pi.md)
    ];

    # Hardware report saved by the installer in /persist/facter.json: copy it next to this
    # file and uncomment, nixpkgs then loads the drivers that machine needs (docs/install.md).
    # facter = ./facter.json;

    # Apps for every user: an app of modules/apps/<name> with its config, else any nixpkgs
    # package by name ("htop", "tmux"); unfree ones are allowed. Each app stands alone and
    # brings no other: list every one you want. Shell tools usually go in the person's apps.
    apps = [
      # "hyprland" # the Hyprland window manager through uwsm, with a polkit agent, a Nerd Font and the key binds of the apps below (docs/desktop.md)
      # "waybar" # a top bar: workspaces, clock, tray, volume, network, battery (docs/desktop.md)
      # "fuzzel" # an app launcher on Super+D, and the clipboard history on Super+C (docs/desktop.md)
      # "kitty" # a terminal, the default one of Super+Return (docs/desktop.md)
      # "mako" # a notification daemon (docs/desktop.md)
      # "lock" # hyprlock and hypridle: a lock screen on Super+L and after 5 minutes idle (docs/desktop.md)
      # "upgrade-notify" # a desktop notification saying what the last auto-upgrade changed (docs/updates.md)
      # "direnv" # per-project dev shells with nix-direnv (docs/dev.md)
      # "firefox" # Firefox with telemetry off, profile kept (docs/desktop.md)
      # "zsh" # zsh with suggestions, syntax highlighting and a shared history; the login shell of whoever lists it (docs/apps.md)
      # "starship" # the starship prompt in plain-text symbols (docs/apps.md)
      # "git" # git with the settings git's own developers use; the person's name and email are the author (docs/apps.md)
      # "delta" # delta, a pager for git diffs and blame (docs/apps.md)
      # "neovim" # neovim as $EDITOR, with the compiler, tree-sitter, ripgrep and fd most configs expect (docs/apps.md)
      # "fzf" # fuzzy finder on Ctrl+T, Ctrl+R, Alt+C, listed by fd so .gitignore is respected (docs/apps.md)
      # "zoxide" # `z` jumps to a directory you visited (docs/apps.md)
      # "eza" # file listings with git status, as ll, la, lt (docs/apps.md)
      # "bat" # bat, a `cat` with syntax highlighting (docs/apps.md)
      # "btop" # a resource monitor (docs/apps.md)
      # "stow" # links your dotfiles repository, ~/dotfiles by default, into your home at each switch (docs/apps.md)
      "nix-index" # nix-locate finds the package of a file, and `, <command>` runs a command without installing it (docs/dev.md)
      # "gh" # the GitHub CLI, its login kept with wipe-home (docs/apps.md)
      # "steam" # Steam with Proton-GE and protontricks, its library kept (docs/specialisations.md)
      # "gamemode" # Feral GameMode, for games started with `gamemoderun` (docs/specialisations.md)
      # "mangohud" # an FPS and frame time overlay, Shift+F2 logs a run (docs/specialisations.md)
    ];

    # Variants of the machine, each one this host plus its own lists, chosen at boot or
    # switched to by `update -s <name>` (docs/specialisations.md).
    # specialisations.maintenance.features = [ "nix-ld" ];

    # Where the machine updates from (docs/updates.md): a flake reference such as
    # "github:<you>/<fork>", whose deploy/<host> branch auto-upgrade pulls once CI passed.
    flake = "github:alice/nixos";
    # mirrors = [ "https://codeberg.org/alice/nixos.git" ]; # other copies of your fork, tried in order when the first is unreachable
    # signers = [ "ssh-ed25519 AAAA... deploy" ]; # keys signing deploy/<host>: auto-upgrade applies only what they signed, `nix run .#init` writes yours

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

    # Python lines appended to this host's boot test, run against the `machine` node.
    testScript = ''
      machine.wait_for_open_port(22)
    '';
  };

  # Hosts whose installer ISO CI installs and boots end to end (docs/install.md).
  installer.testedHosts = [ "example-server" ];
}
