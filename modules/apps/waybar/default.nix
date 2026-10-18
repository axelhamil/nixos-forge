{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.waybar.homeManager =
    { config, lib, ... }:
    {
      programs.waybar = mkDefaults {
        enable = true;
        systemd.enable = true;

        settings.main = {
          layer = "top";
          position = "top";
          height = 30;
          modules-left = lib.optional config.wayland.windowManager.hyprland.enable "hyprland/workspaces";
          modules-center = [ "clock" ];
          modules-right = [
            "tray"
            "idle_inhibitor"
            "wireplumber"
            "network"
            "battery"
          ];
          clock = {
            format = "{:%a %d %b  %H:%M}";
            tooltip-format = "<tt>{calendar}</tt>";
          };
          idle_inhibitor = {
            format = "{icon}";
            format-icons = {
              activated = "awake";
              deactivated = "idle";
            };
          };
          wireplumber = {
            format = "vol {volume}%";
            format-muted = "muted";
            on-click = "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle";
          };
          network = {
            format-wifi = "{essid}";
            format-ethernet = "wired";
            format-disconnected = "offline";
            tooltip-format = "{ifname} {ipaddr}";
          };
          battery = {
            format = "bat {capacity}%";
            format-charging = "charging {capacity}%";
            states = {
              warning = 20;
              critical = 10;
            };
          };
          tray.spacing = 8;
        };
      };
    };
}
