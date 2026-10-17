{
  features.auto-upgrade =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      inherit (config.update) flake mirrors;
      inherit (config) autoUpgrade;

      checkout = flake != null && lib.hasPrefix "/" flake;
      query = flake != null && !checkout && lib.hasInfix "?" flake;
      github = if flake == null then null else lib.match "github:([^/?]+)/([^/?]+).*" flake;
      gitHttps = if flake == null then null else lib.match "git\\+(https://[^?#]+).*" flake;

      primary =
        if github != null then
          [ "https://github.com/${lib.elemAt github 0}/${lib.elemAt github 1}.git" ]
        else if gitHttps != null then
          [ (lib.head gitHttps) ]
        else
          [ ];

      auto-upgrade = pkgs.writeShellApplication {
        name = "auto-upgrade";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.git
          pkgs.jq
          pkgs.nvd
          pkgs.openssh
          config.nix.package
        ];
        runtimeEnv = {
          HOST = config.networking.hostName;
          CHECKOUT = if checkout then flake else "";
          SOURCES = lib.concatLines (primary ++ mirrors);
          OPERATION = autoUpgrade.operation;
          REBUILD = autoUpgrade.rebuildCommand;
          ALLOW_LOCAL_BUILD = lib.boolToString autoUpgrade.allowLocalBuild;
        };
        text = builtins.readFile ./auto-upgrade.sh;
      };
    in
    {
      assertions = [
        {
          assertion = !query;
          message = "auto-upgrade fetches the deploy/<host> branch of the repository in the host's `flake` field, which cannot carry a query such as `?host=` or `?dir=`; got \"${flake}\". Use a plain `github:<owner>/<repo>`, or the repository's URL in `mirrors`.";
        }
        {
          assertion = checkout || primary != [ ];
          message = "auto-upgrade pulls from the host's `flake` field: a checkout on the machine (its `origin` remote), `github:<owner>/<repo>` or `git+https://...`; got ${
            if flake == null then "none" else "\"${flake}\""
          }.";
        }
      ];

      environment.persistence."/persist".directories = [ "/var/lib/auto-upgrade" ];

      environment.systemPackages = [ pkgs.nvd ];

      systemd.additionalUpstreamSystemUnits = [ "systemd-boot-check-no-failures.service" ];
      systemd.targets.boot-complete.requires = [ "systemd-boot-check-no-failures.service" ];

      systemd.services.auto-upgrade = {
        description = "Pull the version CI validated for this machine and apply it";
        restartIfChanged = false;
        unitConfig.X-StopOnRemoval = false;
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        environment =
          config.nix.envVars
          // {
            HOME = "/root";
            GIT_TERMINAL_PROMPT = "0";
          }
          // config.networking.proxy.envVars;
        serviceConfig = {
          Type = "oneshot";
          StateDirectory = "auto-upgrade";
          ExecStart = lib.getExe auto-upgrade;
        };
      };

      systemd.timers.auto-upgrade = {
        wantedBy = [ "timers.target" ];
        timerConfig = {
          OnCalendar = "04:00";
          RandomizedDelaySec = "45min";
          Persistent = true;
        };
      };
    };
}
