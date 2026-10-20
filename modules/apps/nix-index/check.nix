{ config, ... }:

{
  perSystem = config.helpers.appCheck "nix-index" { testScripts = [ ./test.py ]; };
}
