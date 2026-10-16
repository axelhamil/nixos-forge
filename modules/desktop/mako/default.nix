{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.homeManager.mako =
    { config, ... }:
    let
      inherit (config.theme) palette font;
    in
    {
      services.mako = mkDefaults {
        enable = true;
        settings = {
          font = "${font.sans} ${toString font.size}";
          background-color = "#${palette.background}";
          text-color = "#${palette.text}";
          border-color = "#${palette.accent}";
          border-size = 2;
          border-radius = 8;
          default-timeout = 5000;
          "urgency=critical" = {
            border-color = "#${palette.urgent}";
            default-timeout = 0;
          };
        };
      };
    };
}
