{ config, ... }:

let
  inherit (config.flake.modules) nixos homeManager;
in
{
  flake.modules.nixos.hyprland =
    { lib, pkgs, ... }:
    {
      imports = with nixos; [
        greetd
        lock
      ];

      programs.hyprland = {
        enable = true;
        withUWSM = true;
      };

      greetd.command = "uwsm start hyprland-uwsm.desktop";

      fonts = {
        enableDefaultPackages = lib.mkDefault true;
        packages = [ pkgs.nerd-fonts.jetbrains-mono ];
        fontconfig.defaultFonts.monospace = lib.mkDefault [ "JetBrainsMono Nerd Font" ];
      };

      environment.sessionVariables.NIXOS_OZONE_WL = "1";

      home-manager.sharedModules = [ homeManager.hyprland ];

      virtualisation.vmVariant.virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];
    };
}
