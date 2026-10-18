{ inputs, ... }:

{
  features.disk =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      partitions = config.disko.devices.disk.main.content.partitions;

      public = lib.filterAttrsRecursive (name: _: !lib.hasPrefix "_" name);

      partitionDisk = partition: {
        type = "disk";
        inherit (partition.content) device;
        content = public partition.content;
      };

      formatMount = (import inputs.disko { inherit lib; })._cliFormatMount {
        disko.devices.disk = {
          nixos = partitionDisk partitions.luks;
          esp = partitionDisk partitions.ESP;
        };
      } pkgs;
    in
    {
      system.build.installBesideWindows = pkgs.writeShellApplication {
        name = "install-beside-windows";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.gawk
          pkgs.gptfdisk
          pkgs.parted
          pkgs.util-linux
          config.systemd.package
          config.nix.package
        ];
        runtimeEnv = {
          FORMAT_MOUNT = lib.getExe formatMount;
          ESP_DEVICE = partitions.ESP.content.device;
          LUKS_DEVICE = partitions.luks.content.device;
        };
        text = builtins.readFile ./beside-windows.sh;
      };
    };
}
