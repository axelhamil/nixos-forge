{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  base = {
    services.resolved = {
      enable = true;
      settings.Resolve = mkDefaults {
        FallbackDNS = "";
        LLMNR = false;
      };
    };
  };
}
