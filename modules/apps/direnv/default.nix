{ config, ... }:

let
  inherit (config.helpers) persistInEveryHome;
in
{
  apps.direnv.nixos = {
    imports = [ (persistInEveryHome (home: "${home.xdg.dataHome}/direnv")) ];

    programs.direnv.enable = true;
  };
}
