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

  kernelOf = host: config.flake.nixosConfigurations.${host}.config.boot.kernelPackages.kernel;

  kernelsOn =
    system: lib.unique (lib.mapAttrsToList (name: _: (kernelOf name).drvPath) (hostsOn system));

  kernelPinsOf =
    host:
    map (output: {
      name = "kernel-${host}-${output}";
      path = (kernelOf host).${output}.outPath;
    }) (kernelOf host).outputs;

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
    { pkgs, self', ... }:
    {
      packages =
        lib.mapAttrs (config.helpers.forgeCommand pkgs self'.packages.forge-ci) {
          ci-plan = {
            command = "plan";
            runtimeInputs = [
              pkgs.nix
              pkgs.nix-eval-jobs
            ];
          };
          ci-build = {
            command = "build";
            runtimeInputs = [ pkgs.nix ];
          };
          ci-report.command = "report";
          ci-badges = {
            command = "badges";
            runtimeInputs = [ pkgs.git ];
          };
          ci-flaky = {
            command = "flaky";
            runtimeInputs = [ pkgs.nix ];
          };
          ci-flaky-plan = {
            command = "flaky-plan";
            runtimeInputs = [
              pkgs.nix
              pkgs.nix-eval-jobs
              pkgs.git
            ];
          };
          ci-flaky-report = {
            command = "flaky-report";
            runtimeInputs = [ pkgs.git ];
          };
          reproducibility = {
            command = "reproducibility";
            runtimeInputs = [
              pkgs.nix
              pkgs.diffoscopeMinimal
            ];
          };
          ci-promote = {
            command = "promote";
            runtimeInputs = [
              pkgs.git
              pkgs.openssh
            ];
          };
        }
        // {
          closure-diff = pkgs.writeShellApplication {
            name = "closure-diff";
            runtimeInputs = [
              pkgs.git
              pkgs.nvd
            ];
            text = builtins.readFile ./closure-diff.sh;
          };

          ci-boot-time = pkgs.writeShellApplication {
            name = "ci-boot-time";
            runtimeInputs = [
              pkgs.nix
              pkgs.jq
              pkgs.coreutils
            ];
            text = builtins.readFile ./ci-boot-time.sh;
          };

          ci-mirror = pkgs.writeShellApplication {
            name = "ci-mirror";
            runtimeInputs = [
              pkgs.git
              pkgs.coreutils
            ];
            text = builtins.readFile ./ci-mirror.sh;
          };

          ci-pin-kernels = import ./_ci-pin-kernels.nix {
            inherit pkgs;
            inherit (pkgs) nix cachix;
          };

          sync-develop = pkgs.writeShellApplication {
            name = "sync-develop";
            runtimeInputs = [ pkgs.git ];
            text = builtins.readFile ./sync-develop.sh;
          };
        };
    };
}
