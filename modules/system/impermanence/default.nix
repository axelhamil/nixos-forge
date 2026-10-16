{ inputs, ... }:

{
  flake.modules.nixos.base =
    { lib, config, ... }:
    {
      imports = [ inputs.impermanence.nixosModules.impermanence ];

      options.impermanence.persistPrefix = lib.mkOption {
        type = lib.types.str;
        readOnly = true;
        default = lib.optionalString config.environment.persistence."/persist".enable "/persist";
        description = "Prefix for state read before the impermanence bind mounts exist (activation scripts) or kept outside them: `/persist` when persistence is on, empty otherwise.";
      };

      options.impermanence.wipeHome = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether /home starts empty at every boot. Set by importing `wipe-home`.";
      };

      config.home-manager.sharedModules = [
        {
          home.persistence."/persist" = {
            enable = lib.mkDefault config.impermanence.wipeHome;
            hideMounts = lib.mkDefault true;
            allowTrash = lib.mkDefault true;
          };
        }
      ];

      config.environment.persistence."/persist" = {
        enable = lib.mkDefault false;
        hideMounts = true;

        directories = [
          "/var/log"
          "/var/lib/nixos"
          "/var/lib/systemd"
        ];

        files = [ "/etc/machine-id" ];
      };
    };
}
