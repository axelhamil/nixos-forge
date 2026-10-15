{ config, ... }:

{
  perSystem = config.helpers.profileCheck "laptop" [
    ../desktop/test.py
    ./test.py
  ];
}
