{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.homeManager.waybar =
    { config, ... }:
    let
      inherit (config.theme) palette font;
    in
    {
      programs.waybar = mkDefaults {
        enable = true;
        systemd.enable = true;

        settings.main = {
          layer = "top";
          position = "top";
          height = 30;
          modules-left = [ "hyprland/workspaces" ];
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

        style = ''
          * {
            font-family: "${font.monospace}";
            font-size: ${toString font.size}pt;
            border: none;
            border-radius: 0;
            min-height: 0;
          }

          window#waybar {
            background: #${palette.background};
            color: #${palette.text};
          }

          #workspaces button {
            padding: 0 8px;
            color: #${palette.muted};
          }

          #workspaces button.active {
            color: #${palette.text};
            box-shadow: inset 0 -2px #${palette.accent};
          }

          #workspaces button.urgent {
            color: #${palette.urgent};
          }

          #clock, #tray, #idle_inhibitor, #wireplumber, #network, #battery {
            padding: 0 10px;
          }

          #battery.charging {
            color: #${palette.success};
          }

          #battery.warning {
            color: #${palette.warning};
          }

          #battery.critical, #wireplumber.muted, #network.disconnected {
            color: #${palette.urgent};
          }
        '';
      };
    };
}
