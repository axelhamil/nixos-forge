{ inputs, ... }:

{
  flake.modules.nixos.base =
    { lib, ... }:
    {
      imports = [ inputs.impermanence.nixosModules.impermanence ];

      environment.persistence."/persist" = {
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
