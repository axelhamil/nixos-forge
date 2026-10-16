{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "profile-desktop" {
    nodes.machine = {
      imports = [
        nixos.base
        nixos.desktop
      ];
      virtualisation.cores = 2;
      environment.persistence."/persist".enable = true;
    };
    testScripts = [
      ./test.py
      ./persistence-test.py
    ];
  };
}
