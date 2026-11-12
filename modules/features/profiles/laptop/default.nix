{ config, ... }:

{
  profiles.laptop = {
    imports = [ config.profiles.desktop ];

    services.scx-loader.config.scheds.scx_lavd.auto_mode = [ "--autopower" ];
  };
}
