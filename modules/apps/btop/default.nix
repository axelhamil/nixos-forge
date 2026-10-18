{ lib, ... }:

{
  apps.btop.homeManager.programs.btop.enable = lib.mkDefault true;
}
