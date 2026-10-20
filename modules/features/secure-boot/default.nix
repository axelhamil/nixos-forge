{ inputs, ... }:

{
  features.secure-boot =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    {
      imports = [ inputs.lanzaboote.nixosModules.lanzaboote ];

      boot.loader.systemd-boot.enable = lib.mkForce false;

      boot.lanzaboote = {
        enable = true;
        pkiBundle = "/var/lib/sbctl";
        autoGenerateKeys.enable = true;
        autoEnrollKeys.enable = true;
        bootCounting.initialTries = lib.mkDefault 3;
      };

      systemd.services.fwupd-efi = lib.mkIf config.services.fwupd.enable {
        after = [ "generate-sb-keys.service" ];
      };

      environment.systemPackages = [ pkgs.sbctl ];

      environment.persistence."/persist".directories = [ "/var/lib/sbctl" ];
    };
}
