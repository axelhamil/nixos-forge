{ config, ... }:

let
  inherit (config.helpers) persistInEveryHome;
in
{
  apps.upgrade-notify.nixos =
    { lib, pkgs, ... }:
    let
      upgrade-notify = pkgs.writeShellApplication {
        name = "upgrade-notify";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.jq
          pkgs.libnotify
        ];
        text = builtins.readFile ./upgrade-notify.sh;
      };
    in
    {
      imports = [ (persistInEveryHome (home: "${home.xdg.stateHome}/upgrade-notify")) ];

      systemd.user.services.upgrade-notify = {
        description = "Tell the user what the last auto-upgrade changed";
        wantedBy = [ "graphical-session.target" ];
        after = [ "graphical-session.target" ];
        serviceConfig = {
          Type = "oneshot";
          ExecStart = lib.getExe upgrade-notify;
        };
      };
    };
}
