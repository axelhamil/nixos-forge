{ config, ... }:

{
  profiles.laptop = {
    imports = [ config.profiles.desktop ];

    services.scx.extraArgs = [ "--autopower" ];
  };
}
