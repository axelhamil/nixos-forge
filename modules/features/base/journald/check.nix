{ config, ... }:

{
  perSystem = config.helpers.baseCheck "journald" { testScripts = [ ./test.py ]; };
}
