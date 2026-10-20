{ config, ... }:

{
  perSystem = config.helpers.baseCheck "resolved" {
    machine.specialisation.forked.configuration.services.resolved.settings.Resolve.FallbackDNS =
      "192.0.2.1";
    testScripts = [ ./test.py ];
  };
}
