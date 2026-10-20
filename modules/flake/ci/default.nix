{ lib, config, ... }:

let
  runners = {
    x86_64-linux = "ubuntu-24.04";
    aarch64-linux = "ubuntu-24.04-arm";
  };

  inherit (config.helpers) hostsOn;

  outsideBuild = [
    "treefmt"
    "perf-regression"
  ];

  buildChecksOn =
    system: lib.subtractLists outsideBuild (lib.attrNames config.flake.checks.${system});

  jobsOn =
    system:
    map (check: {
      inherit check system;
      runner = runners.${system};
      retry = config.flake.checks.${system}.${check} ? driver;
      baseInput = config.flake.checks.${system}.${check}.baseInput or null;
    }) (buildChecksOn system);

  kernelsOn =
    system:
    lib.unique (
      lib.mapAttrsToList (
        name: _: config.flake.nixosConfigurations.${name}.config.boot.kernelPackages.kernel.drvPath
      ) (hostsOn system)
    );

  hostNamesOn = system: lib.attrNames (hostsOn system);

  systemsWithHosts = lib.filter (system: hostsOn system != { }) config.systems;

  checks = import ./_checks.nix { inherit lib; };

  hostChecks = lib.mapAttrs (
    name: host: checks.hostChecksOf config.flake.checks.${host.system} name
  ) config.configurations.nixos;

  matrix = lib.concatMap jobsOn config.systems;

  descriptions = lib.genAttrs (lib.unique (
    map (job: job.check) matrix
    ++ [
      "treefmt"
      "perf-regression"
    ]
  )) (checks.describe hostChecks);
in
{
  flake.ci = {
    inherit matrix hostChecks descriptions;
    hosts = lib.genAttrs config.systems hostNamesOn;
    kernels = lib.genAttrs config.systems kernelsOn;
    closureMatrix = map (system: {
      inherit system;
      runner = runners.${system};
    }) systemsWithHosts;
    perfMatrix = map (system: {
      inherit system;
      runner = runners.${system};
      inherit (config.flake.checks.${system}.perf-regression) baseInput;
    }) (lib.filter (system: config.flake.checks.${system} ? perf-regression) config.systems);
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

      packages.ci-plan = import ./_ci-plan.nix {
        inherit pkgs;
        inherit (pkgs) nix-eval-jobs;
      };

      packages.ci-build = import ./_ci-build.nix {
        inherit pkgs;
        inherit (pkgs) nix;
      };

      packages.ci-report = import ./_ci-report.nix { inherit pkgs; };

      packages.ci-badges = import ./_ci-badges.nix { inherit pkgs; };

      packages.ci-boot-time = pkgs.writeShellApplication {
        name = "ci-boot-time";
        runtimeInputs = [
          pkgs.nix
          pkgs.jq
          pkgs.coreutils
        ];
        text = builtins.readFile ./ci-boot-time.sh;
      };

      packages.reproducibility = import ./_reproducibility.nix {
        inherit pkgs;
        inherit (pkgs) nix;
      };

      packages.ci-promote = pkgs.writeShellApplication {
        name = "ci-promote";
        runtimeInputs = [
          pkgs.git
          pkgs.jq
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.openssh
        ];
        text = builtins.readFile ./ci-promote.sh;
      };

      packages.ci-mirror = pkgs.writeShellApplication {
        name = "ci-mirror";
        runtimeInputs = [
          pkgs.git
          pkgs.coreutils
        ];
        text = builtins.readFile ./ci-mirror.sh;
      };

      packages.sync-develop = pkgs.writeShellApplication {
        name = "sync-develop";
        runtimeInputs = [ pkgs.git ];
        text = builtins.readFile ./sync-develop.sh;
      };
    };
}
