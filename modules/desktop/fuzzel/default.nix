{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.homeManager.fuzzel =
    { config, ... }:
    let
      inherit (config.theme) palette font;
      opaque = color: "${color}ff";
    in
    {
      programs.fuzzel = mkDefaults {
        enable = true;
        settings = {
          main = {
            font = "${font.sans}:size=${toString (font.size + 2)}";
            terminal = "ghostty -e";
            launch-prefix = "uwsm app --";
            width = 40;
          };
          colors = {
            background = opaque palette.background;
            text = opaque palette.text;
            prompt = opaque palette.muted;
            input = opaque palette.text;
            match = opaque palette.accent;
            selection = opaque palette.surface;
            selection-text = opaque palette.text;
            selection-match = opaque palette.accent;
            border = opaque palette.accent;
          };
          border = {
            width = 2;
            radius = 8;
          };
        };
      };
    };
}
