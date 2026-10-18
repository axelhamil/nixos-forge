{ config, ... }:

{
  perSystem = config.helpers.featureCheck "secrets" {
    machine =
      { config, ... }:
      {
        environment.persistence."/persist".enable = true;

        sops.defaultSopsFile = ./test/secrets.yaml;
        sops.secrets.greeting = { };

        system.activationScripts.installTestKey = "install -D -m 400 ${./test/key.txt} ${config.sops.age.keyFile}";
        system.activationScripts.setupSecrets.deps = [ "installTestKey" ];
      };
    testScripts = [ ./test.py ];
  };
}
