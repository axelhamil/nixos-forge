{ config, ... }:

{
  flake.modules.nixos.laptop = {
    imports = [ config.flake.modules.nixos.desktop ];

    services.scx.extraArgs = [ "--autopower" ];
  };
}
