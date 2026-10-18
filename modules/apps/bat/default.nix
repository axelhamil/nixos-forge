{ lib, ... }:

{
  apps.bat.homeManager.programs.bat.enable = lib.mkDefault true;
}
