{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (lib) mkOption types;
  inherit (config.flake.modules) homeManager;
  inherit (config.flake.modules.nixos) zsh;
  inherit (config.helpers) appPart;
  inherit (config)
    systems
    apps
    base
    profiles
    features
    hardware
    ;

  fail =
    host: field: message:
    throw "configurations.nixos.${host}.${field}: ${message}";

  known = namespace: lib.concatStringsSep ", " (lib.attrNames namespace);

  pick =
    host: field: namespace: name:
    namespace.${name} or (fail host field "unknown name \"${name}\". Known: ${known namespace}.");

  hardwareModule =
    host: name:
    hardware.${name} or inputs.nixos-hardware.nixosModules.${name} or (fail host "hardware"
      "unknown name \"${name}\": neither a module of modules/features/hardware (${known hardware}) nor a nixos-hardware profile (https://github.com/NixOS/nixos-hardware)."
    );

  configuredApps = lib.filter (name: apps ? ${name});
  packagedApps = lib.filter (name: !(apps ? ${name}));

  appModules = part: names: map (name: appPart name part) (configuredApps names);

  package =
    host: pkgs: name:
    lib.attrByPath (lib.splitString "." name) (fail host "apps"
      "unknown name \"${name}\": no modules/apps/${name} and no pkgs.${name}."
    ) pkgs;

  unfreeName =
    host: system: name:
    lib.getName (package host inputs.nixpkgs.legacyPackages.${system} name);

  userOptions = {
    options = {
      fullName = mkOption {
        type = types.str;
        default = "";
        example = "Alice Example";
        description = "Full name, shown by the greeter and used as git author name.";
      };

      email = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "alice@example.org";
        description = "Email used as git author email.";
      };

      admin = mkOption {
        type = types.bool;
        default = true;
        description = "Whether the user may run `sudo` (member of `wheel`).";
      };

      sshKeys = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [ "ssh-ed25519 AAAA... alice@laptop" ];
        description = "Public SSH keys allowed to log in as this user.";
      };

      home = mkOption {
        type = types.listOf types.str;
        default = [ "shell" ];
        example = [
          "shell"
          "stow"
        ];
        description = "home-manager modules the user imports, by name: the starter's (`modules/home/<name>`) or your own, such as `flake.modules.homeManager.<user>` in `modules/users/<user>`.";
      };

      apps = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [
          "firefox"
          "obsidian"
        ];
        description = "Apps for this user only: an app of `modules/apps/<name>` when there is one, else the nixpkgs package of that name (`kdePackages.kate` works). Unfree packages named here are allowed.";
      };

      groups = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [ "libvirtd" ];
        description = "Extra groups of the user, besides `wheel` (from `admin`) and `networkmanager` (when NetworkManager is on).";
      };
    };
  };

  userModule =
    host: system: name: user:
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      usesZsh = lib.elem "shell" user.home || lib.elem "zsh" user.home;
      packages = packagedApps user.apps;
    in
    {
      imports = lib.optional usesZsh zsh ++ appModules "nixos" user.apps;

      users.users.${name} = {
        isNormalUser = true;
        description = user.fullName;
        extraGroups =
          lib.optional user.admin "wheel"
          ++ lib.optional config.networking.networkmanager.enable "networkmanager"
          ++ user.groups;
        shell = lib.mkIf usesZsh pkgs.zsh;
        openssh.authorizedKeys.keys = user.sshKeys;
        hashedPasswordFile =
          lib.mkIf config.environment.persistence."/persist".enable
            "/persist/passwords/${name}";
      };

      nixpkgs.config.allowUnfreePackages = map (unfreeName host system) packages;

      virtualisation.vmVariant.users.users.${name} = {
        hashedPasswordFile = lib.mkForce null;
        initialPassword = "test";
      };

      home-manager.users.${name} = {
        imports =
          map (pick host "users.${name}.home" homeManager) user.home ++ appModules "homeManager" user.apps;
        home.stateVersion = config.system.stateVersion;
        home.packages = map (package host pkgs) packages;
        programs.git.settings.user = lib.mkIf (user.email != null) {
          name = lib.mkDefault user.fullName;
          email = lib.mkDefault user.email;
        };
      };
    };

  hostOptions =
    { name, config, ... }:
    let
      host = config;
      packages = packagedApps host.apps;
    in
    {
      options = {
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
            "hyprland"
            "docker"
          ];
          description = "NixOS modules the machine turns on, by name: the features of `modules/features/<name>` (`disk`, `secure-boot`, `wipe-home`, `hyprland`, `plymouth`, `docker`, `libvirt`, `ssh`, `secrets`, ...). `disk` is the encrypted btrfs layout with `/` wiped at boot; the disk it erases is chosen at install (installer ISO, `disko-install --disk main`, or `disk.device` in `module`), never in the registry.";
        };

        apps = mkOption {
          type = types.listOf types.str;
          default = [ ];
          example = [
            "firefox"
            "vlc"
          ];
          description = "Apps for every user of the machine: an app of `modules/apps/<name>` when there is one, else the nixpkgs package of that name (`kdePackages.kate` works). Unfree packages named here are allowed; a wrapper whose unfree part is an inner package (`steam`) still needs that name in `nixpkgs.config.allowUnfreePackages`.";
        };

        users = mkOption {
          type = types.attrsOf (types.submodule userOptions);
          default = { };
          description = "Accounts of the machine, by login name. Each one gets home-manager with the `home` modules it lists.";
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

        flake = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "/home/alice/nixos";
          description = "Where the machine updates from: a git checkout on the machine, which the `update` command pulls then switches to, or a flake reference such as `github:alice/nixos`, which the `auto-upgrade` feature needs.";
        };

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
      };

      config.module =
        { lib, pkgs, ... }:
        {
          imports = [
            base
          ]
          ++ lib.optional (host.profile != null) (pick name "profile" profiles host.profile)
          ++ map (pick name "features" features) host.features
          ++ map (hardwareModule name) host.hardware
          ++ appModules "nixos" host.apps
          ++ lib.mapAttrsToList (userModule name host.system) host.users;

          networking.hostName = name;
          update.flake = host.flake;
          hardware.facter.reportPath = lib.mkIf (host.facter != null) host.facter;
          time.timeZone = lib.mkDefault host.timeZone;
          i18n.defaultLocale = lib.mkDefault host.locale;
          services.xserver.xkb.layout = lib.mkDefault host.keyboard;
          console.keyMap = lib.mkDefault host.keyboard;

          environment.systemPackages = map (package name pkgs) packages;
          home-manager.sharedModules = appModules "homeManager" host.apps;
          nixpkgs.config.allowUnfreePackages = map (unfreeName name host.system) packages;

          system.stateVersion = host.stateVersion;
        };
    };
in
{
  options.configurations.nixos = mkOption {
    type = types.lazyAttrsOf (types.submodule hostOptions);
    default = { };
    description = "Machines of the flake, one file each under `modules/hosts/<name>`. Each one gets a nixosConfigurations entry and its checks.";
  };
}
