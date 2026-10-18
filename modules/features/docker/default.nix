{ config, ... }:

let
  inherit (config.helpers) persistInEveryHome;
in
{
  features.docker =
    { config, ... }:
    {
      imports = [ (persistInEveryHome (home: "${home.xdg.dataHome}/docker")) ];

      virtualisation.docker.rootless = {
        enable = true;
        setSocketVariable = true;
        daemon.settings = {
          features.containerd-snapshotter = true;
          log-driver = "local";
        };
      };

      systemd.user.services.docker.unitConfig.ConditionGroup = "users";

      systemd.user.services.docker-prune = {
        description = "Remove unused Docker data older than a week";
        after = [ "docker.service" ];
        requires = [ "docker.service" ];
        unitConfig.ConditionGroup = "users";
        environment.DOCKER_HOST = "unix://%t/docker.sock";
        serviceConfig.ExecStart = "${config.virtualisation.docker.rootless.package}/bin/docker system prune --force --filter until=168h";
        startAt = "weekly";
      };

      systemd.user.timers.docker-prune.timerConfig.Persistent = true;

      systemd.services."user@".serviceConfig.Delegate = "cpu cpuset io memory pids";
    };
}
