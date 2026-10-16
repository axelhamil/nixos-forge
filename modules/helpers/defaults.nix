{ lib, ... }:

{
  helpers.mkDefaults = lib.mapAttrsRecursiveCond (attrs: !lib.isDerivation attrs) (_: lib.mkDefault);
}
