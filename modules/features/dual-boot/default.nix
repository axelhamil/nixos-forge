{
  features.dual-boot =
    {
      lib,
      pkgs,
      config,
      options,
      ...
    }:
    let
      cfg = config.dualBoot;
      espMount = config.boot.loader.efi.efiSysMountPoint;
      lanzaboote = options.boot ? lanzaboote;

      syncWindows = "${config.forge.package}/bin/forge dual-boot sync";

      readOnlyDefaults = "uid=$UID,gid=$GID,ro";
    in
    {
      options.dualBoot = {
        windowsPartition = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          example = "/dev/disk/by-uuid/0123456789ABCDEF";
          description = "Windows partition (`C:`) to mount read-only at `dualBoot.windowsMount` when first read, by label or UUID (`lsblk -o NAME,LABEL,UUID`). Null mounts nothing: the file manager still mounts it on click, read-only too.";
        };

        windowsMount = lib.mkOption {
          type = lib.types.str;
          default = "/mnt/windows";
          description = "Where `dualBoot.windowsPartition` is mounted.";
        };
      };

      config = lib.mkMerge [
        {
          boot.loader.systemd-boot.rebootForBitlocker = true;
          boot.loader.systemd-boot.configurationLimit = lib.mkDefault 3;
          boot.loader.systemd-boot.extraInstallCommands = syncWindows;

          boot.supportedFilesystems.ntfs = true;
          services.udisks2.enable = true;
          services.udisks2.settings."mount_options.conf".defaults = {
            "ntfs:ntfs3_defaults" = readOnlyDefaults;
            "ntfs:ntfs_defaults" = "${readOnlyDefaults},windows_names";
          };

          forge.runtimeInputs = [
            pkgs.efibootmgr
            pkgs.util-linux
          ];
          forge.environment.FORGE_DUAL_BOOT_CONFIG = "${pkgs.writeText "dual-boot.json" (
            builtins.toJSON { inherit espMount; }
          )}";

          environment.systemPackages = [
            pkgs.cryptsetup
            pkgs.efibootmgr
          ];

          systemd.services.dual-boot = {
            description = "Show Windows in the boot menu";
            wantedBy = [ "multi-user.target" ];
            unitConfig.RequiresMountsFor = espMount;
            serviceConfig = {
              Type = "oneshot";
              RemainAfterExit = true;
              PrivateMounts = true;
              ExecStart = syncWindows;
            };
          };
        }

        (lib.optionalAttrs lanzaboote {
          boot.lanzaboote.settings.reboot-for-bitlocker = true;

          system.build.installBootLoader = lib.mkIf config.boot.lanzaboote.enable (
            lib.mkForce (
              pkgs.writeShellScript "install-boot-loader" ''
                set -e
                ${config.boot.loader.external.installHook} "$@"
                ${syncWindows}
              ''
            )
          );
        })

        (lib.mkIf (cfg.windowsPartition != null) {
          fileSystems.${cfg.windowsMount} = {
            device = cfg.windowsPartition;
            fsType = "ntfs3";
            options = [
              "ro"
              "noauto"
              "nofail"
              "x-systemd.automount"
              "x-systemd.idle-timeout=10min"
            ];
          };
        })
      ];
    };
}
