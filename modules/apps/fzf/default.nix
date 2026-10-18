{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.fzf.homeManager =
    { lib, pkgs, ... }:
    let
      fd = lib.getExe pkgs.fd;
      find = type: "${fd} --type ${type} --hidden --exclude .git --strip-cwd-prefix";
    in
    {
      programs.fzf = mkDefaults {
        enable = true;
        defaultCommand = find "f";
        fileWidget.command = find "f";
        changeDirWidget.command = find "d";
      };
    };
}
