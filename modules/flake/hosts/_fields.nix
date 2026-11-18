{
  lib,
  config,
  accountOptions,
}:

let
  inherit (lib) mkOption types;
  inherit (config) systems;

  specialisationOptions = {
    options = {
      features = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [ "libvirt" ];
        description = "Features this variant adds to the host's `features`, by the same names.";
      };

      apps = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [
          "steam"
          "gamemode"
          "mangohud"
        ];
        description = "Apps this variant adds to the host's `apps` for every user, by the same names: an app of `modules/apps/<name>`, else a package of the host's pkgs, overlays included.";
      };

      hardware = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [ "nvidia" ];
        description = "Hardware modules this variant adds to the host's `hardware`, by the same names.";
      };

      module = mkOption {
        type = types.deferredModule;
        default = { };
        description = "Any other NixOS configuration of this variant, on top of the host's.";
      };
    };
  };
in
{
  system = mkOption {
    type = types.enum systems;
    default = "x86_64-linux";
    description = "Platform the host runs on; its checks run on a CI runner of the same platform.";
  };

  stateVersion = mkOption {
    type = types.str;
    example = "26.05";
    description = "NixOS release the machine was first installed with. Set it once and never change it.";
  };

  profile = mkOption {
    type = types.nullOr types.str;
    default = null;
    example = "laptop";
    description = "Role of the machine: `desktop`, `laptop`, `server` or `pi`, the profiles of `modules/features/profiles/<name>`.";
  };

  features = mkOption {
    type = types.listOf types.str;
    default = [ ];
    example = [
      "disk"
      "secure-boot"
      "docker"
    ];
    description = "NixOS modules the machine turns on, by name: the features of `modules/features/<name>` (`disk`, `secure-boot`, `wipe-home`, `plymouth`, `docker`, `libvirt`, `ssh`, `secrets`, ...). `disk` is the encrypted btrfs layout with `/` wiped at boot; the disk it erases is chosen at install (installer ISO, `disko-install --disk main`, or `disk.device` in `module`), never in the registry.";
  };

  apps = mkOption {
    type = types.listOf types.str;
    default = [ ];
    example = [
      "hyprland"
      "firefox"
      "vlc"
    ];
    description = "Apps for every user of the machine: an app of `modules/apps/<name>` with its NixOS and home-manager config when there is one (`hyprland` is the window manager; its bar, launcher, terminal and lock screen are apps of their own), else the package of that name in the host's pkgs, overlays included (`kdePackages.kate` works). Unfree packages named here are allowed; a wrapper whose unfree part is an inner package (`steam`) still needs that name in `nixpkgs.config.allowUnfreePackages`.";
  };

  users = mkOption {
    type = types.attrsOf (types.submodule accountOptions);
    default = { };
    description = "Accounts of the machine, by login name: each one is a person of `modules/users/<name>` (the `users` of the flake), with their identity, apps and home-manager `module`, plus the host's `apps`. The fields below are what changes from one machine to another.";
  };

  hardware = mkOption {
    type = types.listOf types.str;
    default = [ ];
    example = [
      "common-pc-ssd"
      "common-cpu-intel"
    ];
    description = "Hardware of the machine, by name: a module of `modules/features/hardware/<name>` (`nvidia`, `amd-gpu`, `intel-gpu`, `raspberry-pi-4`, `raspberry-pi-5`, ...) or a [nixos-hardware](https://github.com/NixOS/nixos-hardware) profile (`common-pc-ssd`, `framework-13-7040-amd`, ...).";
  };

  facter = mkOption {
    type = types.nullOr types.path;
    default = null;
    example = lib.literalExpression "./facter.json";
    description = "Hardware report of the machine from nixos-facter, which the installer ISO saves in `/persist/facter.json`. Copy it next to the host file: nixpkgs' facter module then turns on the kernel modules, graphics, network and virtualisation support that machine needs.";
  };

  inherit (config.helpers.updateOptions) flake mirrors signers;

  timeZone = mkOption {
    type = types.str;
    default = "UTC";
    example = "Europe/Paris";
    description = "Time zone.";
  };

  locale = mkOption {
    type = types.str;
    default = "en_US.UTF-8";
    example = "fr_FR.UTF-8";
    description = "Language and formats.";
  };

  keyboard = mkOption {
    type = types.str;
    default = "us";
    example = "fr";
    description = "Keyboard layout, for the console and the desktop.";
  };

  specialisations = mkOption {
    type = types.attrsOf (types.submodule specialisationOptions);
    default = { };
    example = lib.literalExpression ''{ gaming.apps = [ "steam" "gamemode" ]; }'';
    description = "Variants of the machine, by name, each one the whole host plus its own `features`, `apps`, `hardware` and `module`. Each variant is an entry of the boot menu, and `forge update -s <name>` switches to it without a reboot (`forge update -S` goes back); a kernel or kernel module it changes takes effect only when it is started from the boot menu. The running variant's name is in `/etc/specialisation`, and `forge update` and `auto-upgrade` keep it.";
  };

  module = mkOption {
    type = types.deferredModule;
    default = { };
    description = "Any other NixOS configuration of the machine. The fields above are written into it.";
  };

  testScript = mkOption {
    type = types.lines;
    default = "";
    description = "Python lines appended to the host boot test, run against the `machine` node.";
  };
}
