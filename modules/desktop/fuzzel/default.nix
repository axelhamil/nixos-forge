{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.homeManager.fuzzel.programs.fuzzel = mkDefaults {
    enable = true;
    settings.main = {
      terminal = "kitty";
      launch-prefix = "uwsm app --";
    };
  };
}
