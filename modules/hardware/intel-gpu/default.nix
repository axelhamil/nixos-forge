{
  flake.modules.nixos.intel-gpu =
    { pkgs, ... }:
    {
      hardware.enableRedistributableFirmware = true;

      hardware.graphics = {
        enable = true;
        extraPackages = [
          pkgs.intel-media-driver
          pkgs.vpl-gpu-rt
        ];
      };
    };
}
