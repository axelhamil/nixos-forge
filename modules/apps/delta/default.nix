{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.delta.homeManager.programs.delta = mkDefaults {
    enable = true;
    enableGitIntegration = true;
    options.navigate = true;
  };
}
