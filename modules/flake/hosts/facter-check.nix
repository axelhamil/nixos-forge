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
  evalChecks.facter = {
    text = nixos.hardware.facter.report.system;
    expect = {
      "hardware.facter is off although the registry field `facter` is set" = nixos.hardware.facter.enable;
      "the kvm virtualisation of the report did not add virtio_gpu to the initrd" =
        lib.elem "virtio_gpu" nixos.boot.initrd.kernelModules;
    };
  };
}
