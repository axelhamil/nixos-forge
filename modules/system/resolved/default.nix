{
  flake.modules.nixos.base = {
    services.resolved = {
      enable = true;
      settings.Resolve = {
        FallbackDNS = "";
        LLMNR = false;
      };
    };
  };
}
