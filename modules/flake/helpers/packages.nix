{ withSystem, ... }:

{
  helpers.flakePackage =
    pkgs: name: withSystem pkgs.stdenv.hostPlatform.system ({ config, ... }: config.packages.${name});
}
