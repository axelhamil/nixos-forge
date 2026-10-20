{ lib, ... }:

let
  inherit (lib) mkOption types;

  person =
    { name, ... }:
    {
      options = {
        fullName = mkOption {
          type = types.str;
          default = "";
          example = "Your Name";
          description = "Full name, shown by the greeter and used as git author name.";
        };

        email = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "you@example.org";
          description = "Email used as git author email.";
        };

        sshKeys = mkOption {
          type = types.listOf types.str;
          default = [ ];
          example = [ "ssh-ed25519 AAAA... <user>@laptop" ];
          description = "Public SSH keys allowed to log in as this person, on every host that gives them an account.";
        };

        u2fKeys = mkOption {
          type = types.listOf (types.strMatching ":?[^:\n]+");
          default = [ ];
          example = [ ":owBYz...,pSJ7...,es256,+presence" ];
          description = "Security keys that log the person in and run `sudo` without a password, on every host with the `fido2` feature, one line of `pamu2fcfg -n -o pam://nixos-forge -i pam://nixos-forge` each. Without a key plugged in, the password still works.";
        };

        apps = mkOption {
          type = types.listOf types.str;
          default = [ ];
          example = [
            "zsh"
            "git"
            "neovim"
            "ripgrep"
          ];
          description = "Apps that follow the person on every host, none by default: an app of `modules/apps/<name>` with its NixOS and home-manager config when there is one (`zsh` also becomes the login shell), else the package of that name in the host's pkgs, overlays included (`kdePackages.kate` works). Unfree packages named here are allowed. A host adds its own with `users.<name>.apps`.";
        };

        module = mkOption {
          type = types.deferredModule;
          default = { };
          apply = module: {
            key = "nixos-forge/users/${name}";
            _class = "homeManager";
            imports = [ module ];
          };
          example = lib.literalExpression ''
            { pkgs, ... }:
            {
              home.packages = [ pkgs.gimp ];
              programs.git.settings.alias.st = "status";
            }
          '';
          description = "The person's own home-manager config, on every host that gives them an account. Several files under `modules/users/<name>` may each set part of it.";
        };
      };
    };
in
{
  options.users = mkOption {
    type = types.attrsOf (types.submodule person);
    default = { };
    description = "People of the flake, one folder each under `modules/users/<name>`. A host gives one an account with `configurations.nixos.<host>.users.<name>`.";
  };
}
