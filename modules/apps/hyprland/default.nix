{ config, ... }:

let
  inherit (config) features;
  inherit (config.helpers) appPart;
in
{
  apps.hyprland.nixos =
    { lib, pkgs, ... }:
    {
      imports = [
        features.greetd
        (appPart "lock" "nixos")
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

      virtualisation.vmVariant.virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];
    };
}
