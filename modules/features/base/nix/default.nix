{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  base =
    { lib, ... }:
    {
      nix.settings = {
        experimental-features = [
          "nix-command"
          "flakes"
        ];
        trusted-users = [ "@wheel" ];
      }
      // mkDefaults { use-xdg-base-directories = true; };

      nix.channel.enable = false;

      nix.optimise.automatic = lib.mkDefault true;

      programs.nh = {
        enable = true;
        clean = {
          enable = true;
          extraArgs = lib.mkDefault "--keep-since 14d --keep 5 --keep-one";
        };
      };
    };
}
