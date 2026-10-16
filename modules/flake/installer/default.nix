{ lib, config, ... }:

let
  inherit (config.helpers) installerSystem installerHostsOn;
in
{
  options.installer.testedHosts = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "example-server" ];
    description = "Hosts whose installer ISO gets a `<host>-installer` check: it boots the ISO, installs on a blank disk and boots the result. Each one builds a whole ISO in CI, so keep it to a light host.";
  };

  config.perSystem =
    { system, ... }:
    {
      packages = lib.mapAttrs' (
        name: _:
        lib.nameValuePair "installer-${name}"
          (installerSystem config.flake.nixosConfigurations.${name} [ ]).config.system.build.isoImage
      ) (installerHostsOn system);
    };
}
