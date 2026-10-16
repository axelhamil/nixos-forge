{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "system-ssh" {
    nodes = {
      server = {
        imports = [
          nixos.base
          nixos.ssh
        ];
        environment.persistence."/persist".enable = true;
        users.users.tester = {
          isNormalUser = true;
          password = "test";
        };
      };

      client = { };
    };
    testScripts = [ ./test.py ];
  };
}
