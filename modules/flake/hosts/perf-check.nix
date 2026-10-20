{ lib, config, ... }:

let
  inherit (config.helpers) hostsOn testRevision baseFlake;

  system = "x86_64-linux";
  rounds = 5;

  measured = {
    imports = [ testRevision ];

    virtualisation.vmVariant =
      {
        lib,
        options,
        modulesPath,
        ...
      }:
      {
        imports = [ "${modulesPath}/testing/test-instrumentation.nix" ];
        config = {
          virtualisation = {
            memorySize = lib.mkForce 2048;
            cores = lib.mkForce 2;
          };
        }
        // lib.optionalAttrs (options ? disko) { disko.enableConfig = false; };
      };
  };

  bootsInVm =
    nixos: nixos.pkgs.stdenv.hostPlatform.system == system && !(nixos.config.system.build ? sdImage);

  candidates = lib.filterAttrs (name: _: bootsInVm config.flake.nixosConfigurations.${name}) (
    hostsOn system
  );

  missingOnBase =
    name:
    if !(baseFlake.nixosConfigurations ? ${name}) then
      "absent"
    else if !(bootsInVm baseFlake.nixosConfigurations.${name}) then
      "no-vm"
    else
      null;

  skipped = lib.filterAttrs (_: reason: reason != null) (
    lib.mapAttrs (name: _: missingOnBase name) candidates
  );

  hosts = removeAttrs candidates (lib.attrNames skipped);

  sideOf =
    pkgs: nixos:
    let
      tested = (nixos.extendModules { modules = [ measured ]; }).config;
    in
    {
      vm = "${tested.system.build.vm}/bin/run-${tested.system.name}-vm";
      closure = "${pkgs.closureInfo { rootPaths = [ tested.system.build.toplevel ]; }}/total-nar-size";
    };

  machinesOf =
    pkgs:
    lib.mapAttrs (name: _: {
      base = sideOf pkgs baseFlake.nixosConfigurations.${name};
      current = sideOf pkgs config.flake.nixosConfigurations.${name};
    }) hosts;
in
{
  perSystem =
    { pkgs, ... }:
    lib.mkIf (pkgs.stdenv.hostPlatform.system == system && candidates != { }) {
      checks.perf-regression =
        (
          if hosts == { } then
            pkgs.writeTextFile {
              name = "perf-regression";
              destination = "/perf.json";
              text = builtins.toJSON {
                inherit rounds skipped;
                hosts = { };
              };
            }
          else
            pkgs.testers.runNixOSTest {
              name = "perf-regression";
              nodes = { };
              testScript = ''
                hosts = ${builtins.toJSON (machinesOf pkgs)}
                skipped = ${builtins.toJSON skipped}
                rounds = ${toString rounds}
                ${builtins.readFile ./measure-boot.py}
                ${builtins.readFile ./perf-test.py}
              '';
            }
        )
        // {
          baseInput = "migration-base";
        };
    };
}
