{
  lib,
  config,
  resolve,
}:

let
  inherit (lib) mkOption types;
  inherit (resolve)
    fail
    known
    appModules
    packagedApps
    package
    unfreeNames
    ;

  bringsZsh = lib.elem "zsh";

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
        description = "Apps for this user on this machine only, added to the person's `apps`: an app of `modules/apps/<name>`, else the package of that name in the host's pkgs, overlays included. Unfree packages named here are allowed.";
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

      nixpkgs.config.allowUnfreePackages = lib.concatMap (
        origin: unfreeNames system config.nixpkgs.overlays origin.app
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
in
{
  inherit accountOptions userModule;
}
