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
    map (
      check:
      let
        vm = config.flake.checks.${system}.${check} ? driver;
      in
      {
        inherit check system vm;
        runner = runners.${system};
        baseInput = config.flake.checks.${system}.${check}.baseInput or null;
      }
    ) (buildChecksOn system);

  kernelsOn =
    system:
    lib.unique (
      lib.mapAttrsToList (
        name: _: config.flake.nixosConfigurations.${name}.config.boot.kernelPackages.kernel.drvPath
      ) (hostsOn system)
    );

  kernelPinsOf =
    host:
    let
      kernel = config.flake.nixosConfigurations.${host}.config.boot.kernelPackages.kernel;
    in
    map (output: {
      name = "kernel-${host}-${output}";
      path = kernel.${output}.outPath;
    }) kernel.outputs;

  kernelPins = lib.concatMap kernelPinsOf (lib.attrNames config.configurations.nixos);

  hostNamesOn = system: lib.attrNames (hostsOn system);

  systemsWithHosts = lib.filter (system: hostsOn system != { }) config.systems;

  checks = import ./_checks.nix { inherit lib; };

  hostChecks = lib.mapAttrs (
    name: host: checks.hostChecksOf config.flake.checks.${host.system} name
  ) config.configurations.nixos;

  matrix = lib.concatMap jobsOn config.systems;

  flakyMatrix = map (job: { inherit (job) check system runner; }) (lib.filter (job: job.vm) matrix);

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
    inherit
      matrix
      flakyMatrix
      hostChecks
      descriptions
      kernelPins
      ;
    hosts = lib.genAttrs config.systems hostNamesOn;
    kernels = lib.genAttrs config.systems kernelsOn;
    closureMatrix = map (system: {
      inherit system;
      runner = runners.${system};
    }) systemsWithHosts;
    perfMatrix = map (system: {
      inherit system;
      runner = runners.${system};
      inherit (config.flake.checks.${system}.perf-regression) baseInput timeout;
    }) (lib.filter (system: config.flake.checks.${system} ? perf-regression) config.systems);
  };

  perSystem =
    { pkgs, ... }:
    let
      flakyHistory = import ./_ci-flaky-history.nix {
        inherit pkgs;
        inherit (pkgs) nix nix-eval-jobs;
      };
    in
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

      packages.ci-flaky = import ./_ci-flaky.nix {
        inherit pkgs;
        inherit (pkgs) nix;
      };

      packages.ci-flaky-plan = flakyHistory.plan;

      packages.ci-flaky-report = flakyHistory.report;

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

      packages.ci-pin-kernels = import ./_ci-pin-kernels.nix {
        inherit pkgs;
        inherit (pkgs) nix cachix;
      };

      packages.sync-develop = pkgs.writeShellApplication {
        name = "sync-develop";
        runtimeInputs = [ pkgs.git ];
        text = builtins.readFile ./sync-develop.sh;
      };
    };
}
