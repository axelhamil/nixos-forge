{ inputs, ... }:

let
  source = inputs.migration-base;
in
{
  helpers.baseFlake = builtins.getFlake "path:${builtins.unsafeDiscardStringContext source.outPath}?narHash=${source.narHash}";
}
