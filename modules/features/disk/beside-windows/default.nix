{
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers) flakePackage wrapForge;
in
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

      forge = flakePackage pkgs "forge";
    in
    {
      system.build.installBesideWindows = wrapForge pkgs forge {
        runtimeInputs = [
          pkgs.coreutils
          pkgs.gptfdisk
          pkgs.parted
          pkgs.util-linux
          config.systemd.package
          config.nix.package
        ];
        environment.FORGE_BESIDE_WINDOWS_CONFIG = "${pkgs.writeText "beside-windows.json" (
          builtins.toJSON {
            formatMount = lib.getExe formatMount;
            espDevice = partitions.ESP.content.device;
            luksDevice = partitions.luks.content.device;
          }
        )}";
      };
    };
}
