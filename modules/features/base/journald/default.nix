{
  base =
    { lib, ... }:
    {
      services.journald.settings.Journal.SystemMaxUse = lib.mkDefault "500M";
    };
}
