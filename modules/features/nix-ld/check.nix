{ config, ... }:

{
  perSystem = config.helpers.featureCheck "nix-ld" { testScripts = [ ./test.py ]; };
}
