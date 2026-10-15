{ lib, ... }:

{
  helpers.mkDefaults = lib.mapAttrsRecursive (_: lib.mkDefault);
}
