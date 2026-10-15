{ lib, config, ... }:

{
  perSystem =
    { system, ... }:
    {
      checks =
        lib.mapAttrs' (name: host: lib.nameValuePair "${name}-toplevel" host.config.system.build.toplevel)
          (
            lib.filterAttrs (
              _: host: host.pkgs.stdenv.hostPlatform.system == system
            ) config.flake.nixosConfigurations
          );
    };
}
