{
  apps.hyprland.nixos =
    {
      lib,
      pkgs,
      options,
      ...
    }:
    {
      config = lib.mkMerge [
        {
          programs.hyprland = {
            enable = true;
            withUWSM = true;
          };

          fonts = {
            enableDefaultPackages = lib.mkDefault true;
            packages = [ pkgs.nerd-fonts.jetbrains-mono ];
            fontconfig.defaultFonts.monospace = lib.mkDefault [ "JetBrainsMono Nerd Font" ];
          };

          environment.sessionVariables.NIXOS_OZONE_WL = "1";

          virtualisation.vmVariant.virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];
        }
        (lib.optionalAttrs (options ? greetd) {
          greetd.command = "uwsm start hyprland-uwsm.desktop";
        })
      ];
    };
}
