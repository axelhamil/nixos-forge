{ lib, config, ... }:

let
  runners = {
    x86_64-linux = "ubuntu-24.04";
    aarch64-linux = "ubuntu-24.04-arm";
  };

  inherit (config.helpers) hostsOn;

  buildChecksOn = system: lib.remove "treefmt" (lib.attrNames config.flake.checks.${system});

  jobsOn =
    system:
    map (check: {
      inherit check system;
      runner = runners.${system};
    }) (buildChecksOn system);

  hostNamesOn = system: lib.attrNames (hostsOn system);

  systemsWithHosts = lib.filter (system: hostsOn system != { }) config.systems;
in
{
  flake.ci = {
    matrix = lib.concatMap jobsOn config.systems;
    hosts = lib.genAttrs config.systems hostNamesOn;
    closureMatrix = map (system: {
      inherit system;
      runner = runners.${system};
    }) systemsWithHosts;
  };

  perSystem =
    { pkgs, ... }:
    {
      packages.closure-diff = pkgs.writeShellApplication {
        name = "closure-diff";
        runtimeInputs = [
          pkgs.git
          pkgs.nvd
        ];
        text = builtins.readFile ./closure-diff.sh;
      };

      packages.sync-develop = pkgs.writeShellApplication {
        name = "sync-develop";
        runtimeInputs = [ pkgs.git ];
        text = builtins.readFile ./sync-develop.sh;
      };
    };
}
