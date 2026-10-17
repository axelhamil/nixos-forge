{ config, ... }:

{
  perSystem = config.helpers.baseCheck "nix" { testScripts = [ ./test.py ]; };
}
