{
  lib,
  inputs,
  options,
  ...
}:

let
  hostType = options.configurations.nixos.type.nestedTypes.elemType;

  host =
    (lib.evalModules {
      modules = hostType.getSubModules ++ [
        {
          _module.args.name = "facter-fixture";
          stateVersion = "26.05";
          facter = "${inputs.nixpkgs}/nixos/tests/facter/facter.json";
        }
      ];
    }).config;

  nixos =
    (inputs.nixpkgs.lib.nixosSystem {
      modules = [
        host.module
        { nixpkgs.hostPlatform = host.system; }
      ];
    }).config;
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks.facter =
        assert lib.assertMsg nixos.hardware.facter.enable
          "facter check: hardware.facter is off although the registry field `facter` is set";
        assert lib.assertMsg (lib.elem "virtio_gpu" nixos.boot.initrd.kernelModules)
          "facter check: the kvm virtualisation of the report did not add virtio_gpu to the initrd";
        pkgs.writeText "facter" nixos.hardware.facter.report.system;
    };
}
