{ self, ... }:

{
  base =
    {
      lib,
      pkgs,
      config,
      options,
      ...
    }:
    let
      inherit (config.update) flake;

      checkout = flake != null && lib.hasPrefix "/" flake;
      segments = lib.tail (lib.splitString "/" flake);
      reserved = [
        "/"
        "/home"
        "/nix"
        "/persist"
      ];
      homeItself = lib.length segments == 2 && lib.head segments == "home";
      normalized = lib.all (segment: segment != "" && segment != "." && segment != "..") segments;
      valid = checkout && normalized && !lib.elem flake reserved && !homeItself;

      inHome = valid && lib.length segments > 2 && lib.head segments == "home";
      owner = lib.elemAt segments 1;
      inHomeOfOwner = lib.concatStringsSep "/" (lib.drop 2 segments);
      onWipedRoot =
        valid
        && !lib.any (dir: lib.hasPrefix "${dir}/" flake) [
          "/home"
          "/nix"
          "/persist"
        ];
      persisted = onWipedRoot || (inHome && config.impermanence.wipeHome);
    in
    {
      options.update = {
        flake = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          example = "/home/alice/nixos";
          description = "Flake the machine updates from: a git checkout on the machine, or a flake reference such as `github:alice/nixos`. A checkout is kept across boots on a wiped `/`, and in a wiped home.";
        };

        mirrors = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          example = [ "https://codeberg.org/alice/nixos.git" ];
          description = "Extra git URLs of the fork, tried in order when the first one is unreachable: `https://`, `ssh://`, an scp-like `user@host:path`, `file://` or an absolute path. `http://` and `git://` are refused, since they are not encrypted. `auto-upgrade` runs as root, so an ssh mirror needs a root SSH key that survives a reboot and the mirror's host in `programs.ssh.knownHosts`, and warns otherwise.";
        };

        signers = lib.mkOption {
          type = lib.types.listOf (
            lib.types.strMatching "(ssh|ecdsa|sk)-[a-z0-9@.-]+ [A-Za-z0-9+/]+=*( [^\n]*)?"
          );
          default = [ ];
          example = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyOfTheDeploySecret deploy" ];
          description = "SSH public keys that sign deploy commits: when set, `auto-upgrade` applies only a `deploy/<host>` commit signed by one of them for this host.";
        };
      };

      options.autoUpgrade = {
        operation = lib.mkOption {
          type = lib.types.enum [
            "boot"
            "switch"
          ];
          default = "boot";
          description = "How `auto-upgrade` applies a new version: `boot` makes it the default for the next boot, `switch` also starts it now.";
        };

        allowLocalBuild = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Whether `auto-upgrade` may build what the binary cache lacks on the machine itself; when false it waits for the cache.";
        };

        rebuildCommand = lib.mkOption {
          type = lib.types.str;
          default = lib.getExe config.system.build.nixos-rebuild;
          defaultText = lib.literalExpression "lib.getExe config.system.build.nixos-rebuild";
          internal = true;
          description = "Command `auto-upgrade` runs to build and apply a version.";
        };
      };

      config = lib.mkMerge [
        { system.configurationRevision = lib.mkDefault (self.rev or self.dirtyRev or null); }

        (lib.mkIf (flake != null) (
          lib.mkMerge [
            {
              assertions = [
                {
                  assertion = !checkout || valid;
                  message = "flake = ${builtins.toJSON flake}: a checkout path must be normalized (no `.`, `..`, `//` or trailing `/`) and be a directory of its own, not ${lib.concatStringsSep ", " reserved} or a home itself (`/home/<user>`).";
                }
              ];

              programs.nh.flake = flake;

              environment.systemPackages = [
                (pkgs.writeShellApplication {
                  name = "update";
                  runtimeInputs = [
                    pkgs.git
                    config.programs.nh.package
                  ];
                  runtimeEnv.FLAKE = flake;
                  text = builtins.readFile ./update.sh;
                })
              ];
            }

            (lib.mkIf onWipedRoot { environment.persistence."/persist".directories = [ flake ]; })

            (lib.mkIf inHome {
              home-manager.sharedModules = [
                (home: {
                  home.persistence."/persist".directories = lib.mkIf (
                    home.config.home.username == owner
                    && !(home.options ? stow && lib.removeSuffix "/" home.config.stow.directory == flake)
                  ) [ inHomeOfOwner ];
                })
              ];
            })

            (lib.optionalAttrs (options ? disko) {
              disko.tests.extraChecks = lib.mkIf persisted (
                lib.mkMerge [
                  (lib.mkOrder 900 ''
                    update_checkout = ${builtins.toJSON flake}
                    checkout_owner = ${builtins.toJSON (if inHome then owner else "root")}
                    machine.wait_for_unit("multi-user.target")
                    machine.succeed(f"mkdir -p {update_checkout} && touch {update_checkout}/update-marker")
                  '')
                  (lib.mkOrder 1100 (builtins.readFile ./disk-test.py))
                ]
              );
            })
          ]
        ))
      ];
    };
}
