{ config, ... }:

{
  perSystem = config.helpers.baseCheck "nix-index" [ ./test.py ];
}
