{ inputs, ... }:

{
  features.disk =
    { lib, config, ... }:
    let
      btrfsOptions = [
        "compress=zstd"
        "noatime"
      ];

      subvolume = mountpoint: {
        inherit mountpoint;
        mountOptions = btrfsOptions;
      };
    in
    {
      imports = [ inputs.disko.nixosModules.disko ];

      options.disk.device = lib.mkOption {
        type = lib.types.str;
        default = "/dev/null/no-disk-chosen";
        example = "/dev/disk/by-id/nvme-Samsung_SSD_990_PRO_2TB_S7DNNJ0X123456A";
        description = "Whole disk that disko wipes and formats at install. The installed system mounts its partitions by label, so this only matters to the install; the installer ISO and `disko-install --disk main` set it there. The default can never exist, so a forgotten device erases nothing.";
      };

      options.disk.passphraseFile = lib.mkOption {
        type = lib.types.str;
        default = "/tmp/secret.key";
        readOnly = true;
        description = "File disko reads the LUKS passphrase from at install, without a trailing newline.";
      };

      options.disk.unlockTestScript = lib.mkOption {
        type = lib.types.lines;
        readOnly = true;
        default = ''
          unlock_prompt = ${
            builtins.toJSON (
              if config.boot.plymouth.enable then
                "Please enter passphrase for disk"
              else
                "password query on /dev/tty1"
            )
          }
        ''
        + builtins.readFile ./unlock-disk.py;
        description = "Python defining `unlock_disk(machine)` and `test_passphrase` for a test that boots the installed disk and types the passphrase at the prompt this host shows.";
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
                passwordFile = config.disk.passphraseFile;
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

        boot.loader.systemd-boot = {
          enable = lib.mkDefault true;
          editor = false;
          bootCounting = {
            enable = true;
            tries = lib.mkDefault 3;
          };
        };
        boot.loader.efi.canTouchEfiVariables = lib.mkDefault true;

        services.btrfs.autoScrub = {
          enable = true;
          fileSystems = [ "/" ];
        };

        disko.tests = {
          efi = true;
          bootCommands = ''
            ${config.disk.unlockTestScript}
            unlock_disk(machine)
          '';
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
