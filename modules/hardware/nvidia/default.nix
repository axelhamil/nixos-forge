{
  flake.modules.nixos.nvidia = {
    nixpkgs.config.allowUnfreePackages = [
      "nvidia-x11"
      "nvidia-settings"
      "nvidia-persistenced"
    ];

    hardware.graphics.enable = true;
    services.xserver.videoDrivers = [ "nvidia" ];

    hardware.nvidia = {
      open = true;
      powerManagement.enable = true;
    };
  };
}
