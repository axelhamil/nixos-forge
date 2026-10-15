{
  flake.modules.nixos.disk =
    {
      lib,
      config,
      utils,
      ...
    }:
    let
      rootDevice = config.fileSystems."/".device;
      rootDeviceUnit = "${utils.escapeSystemdPath rootDevice}.device";
    in
    lib.mkIf config.disko.enableConfig {
      boot.initrd.systemd.services.rollback-root = {
        description = "Restore / to its blank snapshot";
        wantedBy = [ "initrd.target" ];
        requires = [ rootDeviceUnit ];
        after = [
          rootDeviceUnit
          "systemd-hibernate-resume.service"
        ];
        before = [ "sysroot.mount" ];
        unitConfig.DefaultDependencies = false;
        serviceConfig.Type = "oneshot";
        environment.device = rootDevice;
        script = builtins.readFile ./rollback.sh;
      };
    };
}
