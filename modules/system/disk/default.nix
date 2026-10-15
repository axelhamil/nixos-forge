{ inputs, ... }:

{
  flake.modules.nixos.disk =
    {
      lib,
      config,
      pkgs,
      ...
    }:
    let
      btrfsOptions = [
        "compress=zstd"
        "noatime"
      ];

      luksName = "cryptroot";
      passphraseFile = "/tmp/secret.key";
      testPassphrase = pkgs.writeText "test-passphrase.key" "secretsecret";

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
                name = luksName;
                passwordFile = passphraseFile;
                settings = {
                  allowDiscards = true;
                  crypttabExtraOpts = [ "tpm2-device=auto" ];
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

        disko.tests = {
          extraConfig.boot.initrd = {
            secrets.${passphraseFile} = testPassphrase;
            luks.devices.${luksName}.keyFile = passphraseFile;
          };

          extraChecks = builtins.readFile ./wipe-test.py;
        };

        virtualisation.vmVariant.disko.enableConfig = false;
      };
    };
}
