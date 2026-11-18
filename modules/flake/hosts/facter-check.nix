{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers) registryHost registryMachine;

  host = registryHost "facter-fixture" [
    { facter = "${inputs.nixpkgs}/nixos/tests/facter/facter.json"; }
  ];

  nixos = (registryMachine host [ ]).config;
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
