{ inputs, ... }:

{
  configurations.nixos.example.module = {
    imports = [ inputs.nixos-hardware.nixosModules.common-pc-ssd ];

    disk.device = "/dev/nvme0n1";

    hardware.cpu.intel.updateMicrocode = true;
  };
}
