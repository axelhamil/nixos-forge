{ config, ... }:

let
  inherit (config.helpers) persistInEveryHome;
in
{
  hardware.audio =
    { pkgs, ... }:
    {
      imports = [ (persistInEveryHome (home: "${home.xdg.stateHome}/wireplumber")) ];

      services.pipewire = {
        enable = true;
        alsa.enable = true;
        alsa.support32Bit = pkgs.stdenv.hostPlatform.isx86_64;
        pulse.enable = true;
      };

      security.rtkit.enable = true;
    };
}
