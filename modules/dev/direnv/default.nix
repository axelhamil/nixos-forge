{ config, ... }:

let
  inherit (config.helpers) persistInEveryHome;
in
{
  flake.modules.nixos.direnv = {
    imports = [ (persistInEveryHome (home: "${home.xdg.dataHome}/direnv")) ];

    programs.direnv.enable = true;
  };
}
