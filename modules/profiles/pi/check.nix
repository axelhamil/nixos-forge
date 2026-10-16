{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "profile-pi" {
    nodes.machine = {
      imports = [
        nixos.base
        nixos.pi
      ];
      nix.buildMachines = [
        {
          hostName = "builder.example";
          system = "aarch64-linux";
          sshUser = "builder";
        }
      ];
      virtualisation.cores = 2;
    };
    testScripts = [
      ../server/test.py
      ./test.py
    ];
  };
}
