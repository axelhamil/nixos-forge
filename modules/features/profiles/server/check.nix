{ config, ... }:

{
  perSystem = config.helpers.profileCheck "server" { testScripts = [ ./test.py ]; };
}
