{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  flake.modules.nixos.pi =
    { lib, config, ... }:
    {
      imports = [ nixos.server ];

      nix.distributedBuilds = lib.mkDefault (config.nix.buildMachines != [ ]);
      nix.settings.builders-use-substitutes = true;

      security.sudo.wheelNeedsPassword = lib.mkDefault false;

      systemd.settings.Manager.RuntimeWatchdogSec = lib.mkDefault "15s";

      services.journald.settings.Journal.SystemMaxUse = lib.mkDefault "100M";

      documentation.nixos.enable = lib.mkDefault false;
    };
}
