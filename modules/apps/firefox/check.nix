{ config, ... }:

{
  perSystem = config.helpers.appCheck "firefox" { testScripts = [ ./test.py ]; };
}
