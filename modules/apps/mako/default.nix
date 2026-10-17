{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.mako.homeManager.services.mako = mkDefaults {
    enable = true;
    settings = {
      default-timeout = 5000;
      "urgency=critical".default-timeout = 0;
    };
  };
}
