{ config, ... }:

{
  perSystem = config.helpers.profileCheck "server" [ ./test.py ];
}
