{
  flake.modules.nixos.server = {
    services.tuned = {
      enable = true;
      ppdSupport = false;
    };
  };
}
