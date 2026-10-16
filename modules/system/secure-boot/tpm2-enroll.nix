{
  flake.modules.nixos.secure-boot =
    {
      lib,
      pkgs,
      config,
      options,
      utils,
      ...
    }:
    let
      luks = config.disko.devices.disk.main.content.partitions.luks;
      volumeKeyDescription = "tpm2-enroll-${luks.content.name}";
      cryptsetupUnit = "systemd-cryptsetup@${utils.escapeSystemdPath luks.content.name}.service";

      volumeKeyEnv = {
        inherit (luks) device;
        volume_key_description = volumeKeyDescription;
      };

      limitVolumeKeyInputs = [
        pkgs.keyutils
        pkgs.cryptsetup
        pkgs.jq
      ];

      limitVolumeKey = pkgs.writeShellApplication {
        name = "limit-volume-key";
        runtimeInputs = limitVolumeKeyInputs;
        runtimeEnv = volumeKeyEnv // {
          timeout = 300;
        };
        text = builtins.readFile ./limit-volume-key.sh;
      };

      tpm2Enroll = pkgs.writeShellApplication {
        name = "tpm2-enroll";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.keyutils
          pkgs.cryptsetup
          pkgs.jq
          config.systemd.package
        ];
        runtimeEnv = volumeKeyEnv // {
          pcrs = "7+15:sha256=${lib.strings.replicate 64 "0"}";
        };
        text = builtins.readFile ./tpm2-enroll.sh;
      };
    in
    {
      config = lib.optionalAttrs (options ? disko) (
        lib.mkIf config.disko.enableConfig {
          boot.initrd.luks.devices.${luks.content.name}.crypttabExtraOpts = [
            "link-volume-key=@u::%user:${volumeKeyDescription}"
          ];

          boot.initrd.systemd.storePaths = [
            limitVolumeKey
          ]
          ++ map (input: "${lib.getBin input}/bin") limitVolumeKeyInputs;

          boot.initrd.systemd.services.limit-volume-key = {
            description = "Drop or expire the volume key linked for tpm2-enroll";
            wantedBy = [ "initrd.target" ];
            after = [ cryptsetupUnit ];
            before = [ "initrd-switch-root.target" ];
            unitConfig.DefaultDependencies = false;
            serviceConfig = {
              Type = "oneshot";
              ExecStart = lib.getExe limitVolumeKey;
              KeyringMode = "shared";
            };
          };

          systemd.services.tpm2-enroll = {
            description = "Enroll the TPM2 to unlock ${luks.device} once Secure Boot is enforced";
            wantedBy = [ "sysinit.target" ];
            before = [
              "sysinit.target"
              "shutdown.target"
            ];
            after = [
              "tpm2.target"
              "systemd-tpm2-setup.service"
              "sys-firmware-efi-efivars.mount"
            ];
            conflicts = [ "shutdown.target" ];
            unitConfig.DefaultDependencies = false;
            serviceConfig = {
              Type = "oneshot";
              RemainAfterExit = true;
              ExecStart = lib.getExe tpm2Enroll;
              KeyringMode = "shared";
              RuntimeDirectory = "tpm2-enroll";
              RuntimeDirectoryMode = "0700";
              UMask = "0077";
            };
          };
        }
      );
    };
}
