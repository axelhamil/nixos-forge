{
  features.auto-upgrade =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      inherit (config.update) flake mirrors signers;
      inherit (config) autoUpgrade;

      stateDir = "/var/lib/${autoUpgrade.stateDirectory}";

      checkout = flake != null && lib.hasPrefix "/" flake;
      query = flake != null && !checkout && lib.hasInfix "?" flake;
      github = if flake == null then null else lib.match "github:([^/?]+)/([^/?]+).*" flake;
      gitHttps = if flake == null then null else lib.match "git\\+(https://[^?#]+).*" flake;

      matches = patterns: url: lib.any (pattern: lib.match pattern url != null) patterns;

      fetchable = matches [
        "(https|file)://.+"
        "/.+"
      ];

      overSsh =
        url:
        lib.hasPrefix "ssh://" url || !(lib.hasInfix "://" url) && matches [ "([^@/:]+@)?[^/:]+:.+" ] url;

      sshMirrors = lib.filter overSsh mirrors;

      cleartextMirrors = lib.filter (url: !fetchable url && !overSsh url) mirrors;

      quoted = lib.concatMapStringsSep ", " (url: "\"${url}\"");

      primary =
        if github != null then
          [ "https://github.com/${lib.elemAt github 0}/${lib.elemAt github 1}.git" ]
        else if gitHttps != null then
          [ (lib.head gitHttps) ]
        else
          [ ];

      allowedSigners = pkgs.writeText "allowed-signers" (
        lib.concatMapStrings (key: "deploy namespaces=\"git\" ${key}\n") signers
      );

      settings = pkgs.writeText "auto-upgrade.json" (
        builtins.toJSON {
          host = config.networking.hostName;
          inherit stateDir;
          checkout = if checkout then flake else null;
          sources = primary ++ mirrors;
          inherit (autoUpgrade) operation rebuildCommand allowLocalBuild;
          allowedSigners = if signers == [ ] then null else "${allowedSigners}";
        }
      );
    in
    {
      warnings =
        lib.optional (sshMirrors != [ ])
          "update.mirrors: auto-upgrade runs as root with HOME=/root, so the mirrors ${quoted sshMirrors} answer only with a root SSH key that survives a reboot (a persisted /root/.ssh or a sops-nix key) and their host in programs.ssh.knownHosts; without them, use their https:// URL. See docs/explanation/updates.md, A machine upgrades on its own.";

      assertions = [
        {
          assertion = cleartextMirrors == [ ];
          message = "update.mirrors: auto-upgrade fetches only over https://, ssh://, an scp-like user@host:path, file:// or an absolute path, so that no one on the network path can choose what this machine builds; got ${quoted cleartextMirrors}. http:// and git:// are not encrypted.";
        }
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

      environment.persistence."/persist".directories = [ stateDir ];

      environment.systemPackages = [ pkgs.nvd ];

      forge.environment.FORGE_AUTO_UPGRADE_CONFIG = "${settings}";
      forge.runtimeInputs = [
        pkgs.git
        pkgs.nvd
        pkgs.openssh
        config.nix.package
        config.systemd.package
      ];

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
          StateDirectory = autoUpgrade.stateDirectory;
          ExecStart = "${lib.getExe config.forge.package} auto-upgrade";
        };
      };

      systemd.timers.auto-upgrade = {
        wantedBy = [ "timers.target" ];
        timerConfig = {
          OnCalendar = lib.mkDefault "04:00";
          RandomizedDelaySec = lib.mkDefault "45min";
          Persistent = true;
        };
      };
    };
}
