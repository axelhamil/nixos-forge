{ config, ... }:

let
  inherit (config.helpers) flakePackage relativeToHome;
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
      forge-apps = flakePackage pkgs "forge-apps";

      upgrade-notify = pkgs.writeShellApplication {
        name = "upgrade-notify";
        runtimeInputs = [ pkgs.libnotify ];
        text = ''exec ${lib.getExe' forge-apps "upgrade-notify"} "$@"'';
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
