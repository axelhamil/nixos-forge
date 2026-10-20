{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (lib) mkOption types;
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
    host: field: name:
    hardware.${name} or inputs.nixos-hardware.nixosModules.${name} or (fail host field
      "unknown name \"${name}\": neither a module of modules/features/hardware (${known hardware}) nor a nixos-hardware profile (https://github.com/NixOS/nixos-hardware)."
    );

  configuredApps = lib.filter (name: apps ? ${name});
  packagedApps = lib.filter (name: !(apps ? ${name}));

  appModules = part: names: map (name: appPart name part) (configuredApps names);

  package =
    where: pkgs: name:
    lib.attrByPath (lib.splitString "." name)
      (throw "${where}: unknown name \"${name}\": neither an app of modules/apps (${known apps}) nor a nixpkgs package; any attribute path of pkgs works, such as \"kdePackages.kate\".")
      pkgs;

  unfreeName =
    where: system: name:
    lib.getName (package where inputs.nixpkgs.legacyPackages.${system} name);

  bringsZsh = lib.elem "zsh";

  selection =
    host: system: prefix: chosen:
    let
      field = name: "${prefix}${name}";
      where = "configurations.nixos.${host}.${field "apps"}";
      packages = packagedApps chosen.apps;
    in
    { pkgs, ... }:
    {
      imports =
        map (pick host (field "features") features) chosen.features
        ++ map (hardwareModule host (field "hardware")) chosen.hardware
        ++ appModules "nixos" chosen.apps;

      environment.systemPackages = map (package where pkgs) packages;
      home-manager.sharedModules = appModules "homeManager" chosen.apps;
      nixpkgs.config.allowUnfreePackages = map (unfreeName where system) packages;
    };

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
        description = "Apps this variant adds to the host's `apps` for every user, by the same names: an app of `modules/apps/<name>`, else a nixpkgs package.";
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

  person =
    host: name:
    config.users.${name} or (fail host "users"
      "unknown person \"${name}\": no modules/users/${name}. Known: ${known config.users}."
    );

  accountOptions = {
    options = {
      admin = mkOption {
        type = types.bool;
        default = true;
        description = "Whether the user may run `sudo` (member of `wheel`).";
      };

      apps = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [
          "stow"
          "kdePackages.kate"
        ];
        description = "Apps for this user on this machine only, added to the person's `apps`: an app of `modules/apps/<name>`, else the nixpkgs package of that name. Unfree packages named here are allowed.";
      };

      groups = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [ "libvirtd" ];
        description = "Extra groups of the user, besides `wheel` (from `admin`) and `networkmanager` (when NetworkManager is on).";
      };
    };
  };

  securityKeys =
    name: user:
    { lib, options, ... }:
    {
      config = lib.optionalAttrs (options ? fido2) {
        fido2.u2fKeys.${name} = user.u2fKeys;
      };
    };

  userModule =
    host: hostApps: system: name: account:
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      user = person host name;
      userApps = user.apps ++ account.apps;
      packages =
        lib.concatMap (origin: map (app: origin // { inherit app; }) (packagedApps origin.apps))
          [
            {
              where = "users.${name}.apps (modules/users/${name})";
              inherit (user) apps;
            }
            {
              where = "configurations.nixos.${host}.users.${name}.apps";
              inherit (account) apps;
            }
          ];
    in
    {
      imports = appModules "nixos" userApps ++ [ (securityKeys name user) ];

      users.users.${name} = {
        isNormalUser = true;
        description = user.fullName;
        extraGroups =
          lib.optional account.admin "wheel"
          ++ lib.optional config.networking.networkmanager.enable "networkmanager"
          ++ account.groups;
        shell = lib.mkIf (bringsZsh (hostApps ++ userApps)) pkgs.zsh;
        openssh.authorizedKeys.keys = user.sshKeys;
        hashedPasswordFile =
          lib.mkIf config.environment.persistence."/persist".enable
            "/persist/passwords/${name}";
      };

      nixpkgs.config.allowUnfreePackages = map (
        origin: unfreeName origin.where system origin.app
      ) packages;

      virtualisation.vmVariant.users.users.${name} = {
        hashedPasswordFile = lib.mkForce null;
        initialPassword = "test";
      };

      home-manager.users.${name} = {
        imports = appModules "homeManager" userApps ++ [ user.module ];
        home.stateVersion = config.system.stateVersion;
        home.packages = map (origin: package origin.where pkgs origin.app) packages;
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
          description = "Apps for every user of the machine: an app of `modules/apps/<name>` with its NixOS and home-manager config when there is one (`hyprland` is the window manager; its bar, launcher, terminal and lock screen are apps of their own), else the nixpkgs package of that name (`kdePackages.kate` works). Unfree packages named here are allowed; a wrapper whose unfree part is an inner package (`steam`) still needs that name in `nixpkgs.config.allowUnfreePackages`.";
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

        flake = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "/home/<user>/nixos";
          description = "Where the machine updates from: a git checkout on the machine, which the `update` command pulls then switches to, or a flake reference such as `github:<owner>/<repo>`. The `auto-upgrade` feature pulls the `deploy/<host>` branch of that fork (of the checkout's `origin` remote).";
        };

        mirrors = mkOption {
          type = types.listOf types.str;
          default = [ ];
          example = [ "https://codeberg.org/<owner>/<repo>.git" ];
          description = "Extra git URLs of the fork, tried in order when the first one is unreachable: `https://`, `ssh://`, an scp-like `user@host:path`, `file://` or an absolute path. `http://` and `git://` are refused, since they are not encrypted.";
        };

        signers = mkOption {
          type = types.listOf types.str;
          default = [ ];
          example = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyOfTheDeploySecret deploy" ];
          description = "SSH public keys that sign the deploy commits CI promotes (the secret `DEPLOY_SIGNING_KEY`, which `nix run .#init` generates). When set, `auto-upgrade` applies a `deploy/<host>` commit only when it carries a valid signature from one of them, and skips a source that serves anything else. Empty, it trusts whatever its sources serve, and `doctor` warns.";
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

        specialisations = mkOption {
          type = types.attrsOf (types.submodule specialisationOptions);
          default = { };
          example = lib.literalExpression ''{ gaming.apps = [ "steam" "gamemode" ]; }'';
          description = "Variants of the machine, by name, each one the whole host plus its own `features`, `apps`, `hardware` and `module`. Each variant is an entry of the boot menu, and `update -s <name>` switches to it without a reboot (`update -S` goes back); a kernel or kernel module it changes takes effect only when it is started from the boot menu. The running variant's name is in `/etc/specialisation`, and `update` and `auto-upgrade` keep it.";
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
        { lib, ... }:
        {
          imports = [
            base
            (selection name host.system "" host)
          ]
          ++ lib.optional (host.profile != null) (pick name "profile" profiles host.profile)
          ++ lib.mapAttrsToList (userModule name host.apps host.system) host.users;

          specialisation = lib.mapAttrs (variant: chosen: {
            configuration = {
              imports = [
                (selection name host.system "specialisations.${variant}." chosen)
                chosen.module
              ];
              environment.etc.specialisation.text = variant;
            };
          }) host.specialisations;

          networking.hostName = name;
          update.flake = host.flake;
          update.mirrors = host.mirrors;
          update.signers = host.signers;
          hardware.facter.reportPath = lib.mkIf (host.facter != null) host.facter;
          time.timeZone = lib.mkDefault host.timeZone;
          i18n.defaultLocale = lib.mkDefault host.locale;
          services.xserver.xkb.layout = lib.mkDefault host.keyboard;
          console.keyMap = lib.mkDefault host.keyboard;

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
