{ config, ... }:

let
  inherit (config.helpers) persistInEveryHome;
in
{
  flake.modules.nixos.wipe-home =
    {
      lib,
      config,
      options,
      utils,
      ...
    }:
    let
      homeDevice = config.fileSystems."/home".device;
      homeDeviceUnit = "${utils.escapeSystemdPath homeDevice}.device";

      tester = "wipe-home-tester";
    in
    {
      imports = [ (persistInEveryHome (home: "${home.xdg.dataHome}/systemd/timers")) ];

      config = lib.mkMerge [
        {
          impermanence.wipeHome = true;

          assertions = [
            {
              assertion = options ? disko;
              message = "wipe-home rolls back the @home subvolume of the disk module: import disk too.";
            }
          ];
        }

        (lib.optionalAttrs (options ? disko) {
          fileSystems."/home".neededForBoot = lib.mkIf config.disko.enableConfig true;

          boot.initrd.systemd.services.rollback-home = lib.mkIf config.disko.enableConfig {
            description = "Move /home aside and start it empty";
            wantedBy = [ "initrd.target" ];
            requires = [ homeDeviceUnit ];
            after = [
              homeDeviceUnit
              "systemd-hibernate-resume.service"
              "rollback-root.service"
            ];
            before = [ "sysroot-home.mount" ];
            unitConfig.DefaultDependencies = false;
            serviceConfig.Type = "oneshot";
            environment.device = homeDevice;
            script = builtins.readFile ./rollback-home.sh;
          };

          disko.tests = {
            extraConfig = {
              users.users.${tester}.isNormalUser = true;
              home-manager.users.${tester}.home = {
                stateVersion = "26.05";
                persistence."/persist".directories = [ "kept" ];
              };
            };

            extraChecks = lib.mkMerge [
              (lib.mkOrder 900 (builtins.readFile ./before-reboot.py))
              (lib.mkOrder 1100 (builtins.readFile ./wipe-home-test.py))
            ];
          };
        })
      ];
    };
}
