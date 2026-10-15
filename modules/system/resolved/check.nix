{ config, ... }:

{
  perSystem = config.helpers.baseCheck "resolved" [ ./test.py ];
}
