{ config, ... }:

let
  inherit (config) profiles;
in
{
  profiles.pi =
    { lib, config, ... }:
    {
      imports = [ profiles.server ];

      nix.distributedBuilds = lib.mkDefault (config.nix.buildMachines != [ ]);
      nix.settings.builders-use-substitutes = true;

      security.sudo.wheelNeedsPassword = lib.mkDefault false;

      systemd.settings.Manager.RuntimeWatchdogSec = lib.mkDefault "15s";

      services.journald.settings.Journal.SystemMaxUse = lib.mkOverride 900 "100M";

      documentation.nixos.enable = lib.mkDefault false;

      autoUpgrade.allowLocalBuild = lib.mkDefault false;
    };
}
