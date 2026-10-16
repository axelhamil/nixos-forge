{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.homeManager.ghostty =
    { config, ... }:
    let
      inherit (config.theme) palette font;
    in
    {
      programs.ghostty = mkDefaults {
        enable = true;
        settings = {
          font-family = font.monospace;
          font-size = font.size + 1;
          inherit (palette) background;
          foreground = palette.text;
          cursor-color = palette.accent;
          selection-background = palette.overlay;
          window-padding-x = 8;
          window-padding-y = 8;
          confirm-close-surface = false;
        };
      };
    };
}
