{ config, ... }:

let
  inherit (config.flake.modules) nixos;
in
{
  perSystem = config.helpers.vmCheck "system-secrets" {
    nodes.machine =
      { config, ... }:
      {
        imports = [
          nixos.base
          nixos.secrets
        ];

        environment.persistence."/persist".enable = true;

        sops.defaultSopsFile = ./test/secrets.yaml;
        sops.secrets.greeting = { };

        system.activationScripts.installTestKey = "install -D -m 400 ${./test/key.txt} ${config.sops.age.keyFile}";
        system.activationScripts.setupSecrets.deps = [ "installTestKey" ];
      };
    testScripts = [ ./test.py ];
  };
}
