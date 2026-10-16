{ config, ... }:

{
  perSystem = config.helpers.appCheck "firefox" [ ./test.py ];
}
