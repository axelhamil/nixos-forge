{ inputs, config, ... }:

{
  flake.nixosConfigurations.desktop = inputs.nixpkgs.lib.nixosSystem {
    modules = with config.flake.modules.nixos; [
      base
      axel
      desktop
      { nixpkgs.hostPlatform = "x86_64-linux"; }
    ];
  };

  flake.modules.nixos.desktop = { pkgs, ... }: {
    networking.hostName = "desktop";
    networking.networkmanager.enable = true;

    boot.loader.systemd-boot.enable = true;
    boot.loader.efi.canTouchEfiVariables = true;
    boot.kernelPackages = pkgs.linuxPackages_latest;

    virtualisation.vmVariant = {
      virtualisation.memorySize = 8192;
      virtualisation.cores = 4;
      users.users.axel.initialPassword = "test";
    };

    system.stateVersion = "26.05";
  };
}
