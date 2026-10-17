{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.fuzzel.homeManager.programs.fuzzel = mkDefaults {
    enable = true;
    settings.main = {
      terminal = "kitty";
      launch-prefix = "uwsm app --";
    };
  };
}
