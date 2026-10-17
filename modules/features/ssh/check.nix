{ config, ... }:

let
  inherit (config) base features;
in
{
  perSystem = config.helpers.vmCheck "feature-ssh" {
    nodes = {
      server = {
        imports = [
          base
          features.ssh
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
