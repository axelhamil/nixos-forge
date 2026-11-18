{ config, ... }:

{
  perSystem =
    { pkgs, self', ... }:
    {
      packages.upstream = config.helpers.forgeCommand pkgs self'.packages.forge "upstream" {
        command = "upstream";
        runtimeInputs = [ pkgs.git ];
      };
    };
}
