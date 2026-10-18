{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.lock.nixos.security.pam.services.hyprlock = { };

  apps.lock.homeManager =
    { config, lib, ... }:
    let
      onHyprland = config.wayland.windowManager.hyprland.enable;
      screen = state: "hyprctl dispatch 'hl.dsp.dpms({ action = \"${state}\" })'";
    in
    {
      programs.hyprlock = mkDefaults {
        enable = true;
        settings = {
          general.hide_cursor = true;
          input-field = {
            size = "300, 50";
            outline_thickness = 2;
            placeholder_text = "Password";
          };
          label = {
            text = "$TIME";
            font_size = 64;
            position = "0, 120";
            halign = "center";
            valign = "center";
          };
        };
      };

      services.hypridle = mkDefaults {
        enable = true;
        settings = {
          general = {
            lock_cmd = "pidof hyprlock || hyprlock";
            before_sleep_cmd = "loginctl lock-session";
          }
          // lib.optionalAttrs onHyprland { after_sleep_cmd = screen "on"; };
          listener = [
            {
              timeout = 300;
              on-timeout = "loginctl lock-session";
            }
          ]
          ++ lib.optional onHyprland {
            timeout = 600;
            on-timeout = screen "off";
            on-resume = screen "on";
          };
        };
      };
    };
}
