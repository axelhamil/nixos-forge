{
  hardware.nvidia =
    { lib, ... }:
    {
      nixpkgs.config.allowUnfreePackages = [
        "nvidia-x11"
        "nvidia-settings"
        "nvidia-persistenced"
      ];

      hardware.graphics.enable = true;
      services.xserver.videoDrivers = [ "nvidia" ];

      hardware.nvidia = {
        open = lib.mkOverride 1100 true;
        powerManagement.enable = lib.mkOverride 1100 true;
      };
    };
}
