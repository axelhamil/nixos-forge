{ config, ... }:

{
  perSystem = config.helpers.baseCheck "nix-index" { testScripts = [ ./test.py ]; };
}
