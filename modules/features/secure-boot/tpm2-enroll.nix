{
  features.secure-boot =
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

      limitVolumeKeyInputs = [
        pkgs.keyutils
        pkgs.cryptsetup
        pkgs.jq
      ];

      limitVolumeKey = pkgs.writeShellApplication {
        name = "limit-volume-key";
        runtimeInputs = limitVolumeKeyInputs;
        runtimeEnv = {
          device = config.forge.luksDevice;
          volume_key_description = volumeKeyDescription;
          timeout = 300;
        };
        text = builtins.readFile ./limit-volume-key.sh;
      };
    in
    {
      config = lib.optionalAttrs (options ? disko) (
        lib.mkIf config.disko.enableConfig {
          forge.enroll = {
            inherit volumeKeyDescription;
            pcrs = "7+15:sha256=${lib.strings.replicate 64 "0"}";
          };

          forge.runtimeInputs = [
            pkgs.keyutils
            pkgs.cryptsetup
            config.systemd.package
          ];

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
            description = "Enroll the TPM2 to unlock ${config.forge.luksDevice} once Secure Boot is enforced";
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
              ExecStart = "${lib.getExe config.forge.package} enroll tpm2";
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
