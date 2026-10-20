{ config, ... }:

let
  inherit (config.helpers) relativeToHome;
in
{
  apps.upgrade-notify.homeManager =
    {
      lib,
      pkgs,
      config,
      ...
    }:
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
      systemd.user.services.upgrade-notify = {
        Unit = {
          Description = "Tell the user what the last auto-upgrade changed";
          After = [ "graphical-session.target" ];
        };
        Service = {
          Type = "oneshot";
          ExecStart = lib.getExe upgrade-notify;
        };
        Install.WantedBy = [ "graphical-session.target" ];
      };

      home.persistence."/persist".directories = [
        (relativeToHome config "${config.xdg.stateHome}/upgrade-notify")
      ];
    };
}
