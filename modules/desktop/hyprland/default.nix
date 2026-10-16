{ config, ... }:

let
  inherit (config.flake.modules) nixos homeManager;
  inherit (config.helpers) persistInEveryHome;
in
{
  flake.modules.nixos.hyprland =
    { pkgs, ... }:
    {
      imports = with nixos; [
        greetd
        theme
        lock
        (persistInEveryHome (home: "${home.xdg.dataHome}/keyrings"))
      ];

      programs.hyprland = {
        enable = true;
        withUWSM = true;
      };

      greetd.command = "uwsm start hyprland-uwsm.desktop";

      xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];

      services.gnome.gnome-keyring.enable = true;
      security.pam.services.greetd.enableGnomeKeyring = true;

      environment.sessionVariables.NIXOS_OZONE_WL = "1";

      home-manager.sharedModules = [ homeManager.hyprland ];

      virtualisation.vmVariant.virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];
    };
}
