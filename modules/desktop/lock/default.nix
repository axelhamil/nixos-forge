{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.nixos.lock.security.pam.services.hyprlock = { };

  flake.modules.homeManager.lock =
    { config, ... }:
    let
      inherit (config.theme) palette font;
      rgb = color: "rgb(${color})";
    in
    {
      programs.hyprlock = mkDefaults {
        enable = true;
        settings = {
          general.hide_cursor = true;
          background.color = rgb palette.background;
          input-field = {
            size = "300, 50";
            outline_thickness = 2;
            outer_color = rgb palette.accent;
            inner_color = rgb palette.surface;
            font_color = rgb palette.text;
            fail_color = rgb palette.urgent;
            placeholder_text = "Password";
          };
          label = {
            text = "$TIME";
            font_family = font.sans;
            font_size = 64;
            color = rgb palette.text;
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
            after_sleep_cmd = "hyprctl dispatch 'hl.dsp.dpms({ action = \"on\" })'";
          };
          listener = [
            {
              timeout = 300;
              on-timeout = "loginctl lock-session";
            }
            {
              timeout = 600;
              on-timeout = "hyprctl dispatch 'hl.dsp.dpms({ action = \"off\" })'";
              on-resume = "hyprctl dispatch 'hl.dsp.dpms({ action = \"on\" })'";
            }
          ];
        };
      };
    };
}
