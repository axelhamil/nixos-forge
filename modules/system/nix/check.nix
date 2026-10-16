{ config, ... }:

{
  perSystem = config.helpers.baseCheck "nix" [ ./test.py ];
}
