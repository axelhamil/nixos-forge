{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.kitty.homeManager.programs.kitty = mkDefaults {
    enable = true;
    settings = {
      window_padding_width = 8;
      confirm_os_window_close = 0;
      enable_audio_bell = false;
    };
  };
}
