{ inputs, ... }:

{
  flake.modules.nixos.disk =
    { lib, config, ... }:
    let
      btrfsOptions = [
        "compress=zstd"
        "noatime"
      ];

      passphraseFile = "/tmp/secret.key";

      subvolume = mountpoint: {
        inherit mountpoint;
        mountOptions = btrfsOptions;
      };
    in
    {
      imports = [ inputs.disko.nixosModules.disko ];

      options.disk.device = lib.mkOption {
        type = lib.types.str;
        description = "Whole disk that disko wipes and formats at install.";
      };

      config = {
        disko.devices.disk.main = {
          type = "disk";
          inherit (config.disk) device;

          content = {
            type = "gpt";

            partitions.ESP = {
              size = "1G";
              type = "EF00";
              content = {
                type = "filesystem";
                format = "vfat";
                mountpoint = "/boot";
                mountOptions = [ "umask=0077" ];
              };
            };

            partitions.luks = {
              size = "100%";
              content = {
                type = "luks";
                name = "cryptroot";
                passwordFile = passphraseFile;
                settings = {
                  allowDiscards = true;
                  crypttabExtraOpts = [
                    "tpm2-device=auto"
                    "tpm2-measure-pcr=yes"
                  ];
                };

                content = {
                  type = "btrfs";
                  extraArgs = [ "-f" ];
                  postCreateHook = builtins.readFile ./blank-snapshot.sh;

                  subvolumes = {
                    "@root" = subvolume "/";
                    "@nix" = subvolume "/nix";
                    "@persist" = subvolume "/persist";
                    "@home" = subvolume "/home";
                    "@snapshots" = {
                      mountpoint = "/.snapshots";
                      mountOptions = btrfsOptions ++ [ "nofail" ];
                    };
                  };
                };
              };
            };
          };
        };

        fileSystems."/persist".neededForBoot = true;
        environment.persistence."/persist".enable = true;
        users.mutableUsers = false;

        boot.loader.systemd-boot.editor = false;

        services.btrfs.autoScrub = {
          enable = true;
          fileSystems = [ "/" ];
        };

        disko.tests = {
          efi = true;
          bootCommands = builtins.readFile ./unlock-disk.py;
          extraChecks = lib.mkMerge [
            (builtins.readFile ./wipe-test.py)
            (lib.mkIf (!config.impermanence.wipeHome) (
              lib.mkOrder 1100 (builtins.readFile ./keep-home-test.py)
            ))
          ];
        };

        virtualisation.vmVariant = {
          disko.enableConfig = false;
          services.btrfs.autoScrub.enable = lib.mkForce false;
        };
      };
    };
}
