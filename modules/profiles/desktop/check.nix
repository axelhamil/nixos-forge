{ config, ... }:

{
  perSystem = config.helpers.profileCheck "desktop" [ ./test.py ];
}
