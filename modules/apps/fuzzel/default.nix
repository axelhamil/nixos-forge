{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.fuzzel.homeManager =
    { config, lib, ... }:
    {
      programs.fuzzel = mkDefaults {
        enable = true;
        settings.main =
          lib.optionalAttrs config.xdg.terminal-exec.enable { terminal = "xdg-terminal-exec"; }
          // lib.optionalAttrs config.wayland.windowManager.hyprland.enable {
            launch-prefix = "uwsm app --";
          };
      };
    };
}
