{ config, ... }:

{
  perSystem = config.helpers.baseCheck "resolved" { testScripts = [ ./test.py ]; };
}
