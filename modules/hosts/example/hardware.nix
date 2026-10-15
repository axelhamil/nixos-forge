{
  configurations.nixos.example.module = {
    disk.device = "/dev/nvme0n1";

    hardware.cpu.intel.updateMicrocode = true;
  };
}
