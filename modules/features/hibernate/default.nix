{
  features.hibernate =
    {
      lib,
      config,
      options,
      ...
    }:
    let
      rollbackOrders =
        service:
        lib.elem "systemd-hibernate-resume.service" (
          config.boot.initrd.systemd.services.${service}.after or [ "systemd-hibernate-resume.service" ]
        );
    in
    {
      options.hibernate = {
        swapSize = lib.mkOption {
          type = lib.types.strMatching "[0-9]+[KMGT]";
          default = "16G";
          example = "32G";
          description = "Size of the swap file that holds the hibernation image, created by disko at install only. Match the RAM of the machine so a full memory still fits.";
        };

        delay = lib.mkOption {
          type = lib.types.str;
          default = "2h";
          example = "45min";
          description = "Time `suspend-then-hibernate` (lid closed) stays suspended before it hibernates (`HibernateDelaySec`).";
        };
      };

      config = lib.mkMerge [
        {
          assertions = [
            {
              assertion = options ? disko;
              message = "hibernate writes its image to a swap file inside the LUKS volume of the disk module: import disk too.";
            }
            {
              assertion = config.boot.initrd.systemd.enable;
              message = "hibernate resumes through the systemd initrd (boot.initrd.systemd.enable).";
            }
            {
              assertion = rollbackOrders "rollback-root" && rollbackOrders "rollback-home";
              message = "The initrd rollback of / and /home must run after systemd-hibernate-resume.service, or it would rewrite the disk under a hibernated system.";
            }
          ];

          systemd.sleep.settings.Sleep.HibernateDelaySec = config.hibernate.delay;

          services.logind.settings.Login.HandleLidSwitch = lib.mkDefault "suspend-then-hibernate";
        }

        (lib.optionalAttrs (options ? disko) {
          disko.devices.disk.main.content.partitions.luks.content.content.subvolumes."@swap" = {
            mountpoint = "/swap";
            mountOptions = [ "noatime" ];
            swap.swapfile = {
              size = config.hibernate.swapSize;
              priority = 0;
            };
          };

          disko.tests.extraChecks = lib.mkOrder 1150 (builtins.readFile ./swap-test.py);
        })
      ];
    };
}
