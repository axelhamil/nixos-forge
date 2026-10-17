{ inputs, config, ... }:

let
  inherit (config.helpers) relativeToHome;
in
{
  base = {
    imports = [ inputs.home-manager.nixosModules.home-manager ];

    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;

      sharedModules = [
        (
          { config, ... }:
          {
            home.persistence."/persist".directories =
              map
                (path: {
                  directory = relativeToHome config path;
                  mode = "0700";
                })
                [
                  "${config.home.homeDirectory}/.ssh"
                  config.programs.gpg.homedir
                  "${config.xdg.configHome}/gh"
                ];
          }
        )
      ];
    };
  };
}
