{ lib, config, ... }:

{
  helpers.relativeToHome = home: path: lib.removePrefix "${home.home.homeDirectory}/" path;

  helpers.persistInEveryHome =
    pathIn:
    { options, ... }:
    {
      config = lib.optionalAttrs (options ? home-manager) {
        home-manager.sharedModules = [
          (home: {
            home.persistence."/persist".directories = [
              (config.helpers.relativeToHome home.config (pathIn home.config))
            ];
          })
        ];
      };
    };
}
