{
  flake.modules.nixos.base = {
    time.timeZone = "Europe/Paris";
    i18n.defaultLocale = "fr_FR.UTF-8";
    console.keyMap = "fr";
    services.xserver.xkb.layout = "fr";
  };
}
