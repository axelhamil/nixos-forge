{ config, ... }:

let
  inherit (config.helpers) persistInEveryHome;
in
{
  features.direnv = {
    imports = [ (persistInEveryHome (home: "${home.xdg.dataHome}/direnv")) ];

    programs.direnv.enable = true;
  };
}
