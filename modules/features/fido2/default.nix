{
  features.fido2 =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      inherit (lib) mkOption types;

      cfg = config.fido2;
      relyingParty = "pam://nixos-forge";

      mappings = lib.concatLines (
        lib.mapAttrsToList (
          user: keys: lib.concatStringsSep ":" ([ user ] ++ map (lib.removePrefix ":") keys)
        ) (lib.filterAttrs (_: keys: keys != [ ]) cfg.u2fKeys)
      );
    in
    {
      options.fido2 = {
        u2fKeys = mkOption {
          type = types.attrsOf (types.listOf (types.strMatching ":?[^:\n]+"));
          default = { };
          example = {
            alice = [ ":owBYz...,pSJ7...,es256,+presence" ];
          };
          description = "Security keys that log each user in and run `sudo` without a password, as printed by `pamu2fcfg -n -o pam://nixos-forge -i pam://nixos-forge`. The registry fills it from each person's `u2fKeys`.";
        };

        unlockLuks = mkOption {
          type = types.bool;
          default = false;
          description = "Unlock every LUKS volume of `boot.initrd.luks.devices` with a security key enrolled by `fido2-enroll`. Set it only once every such volume has a key enrolled: systemd-cryptsetup stops the boot on a volume without one.";
        };
      };

      options.boot.initrd.luks.devices = mkOption {
        type = types.attrsOf (
          types.submodule {
            crypttabExtraOpts = lib.mkIf cfg.unlockLuks [ "fido2-device=auto" ];
          }
        );
      };

      config = {
        security.pam.u2f = {
          enable = true;
          control = lib.mkDefault "sufficient";
          settings = {
            cue = lib.mkDefault true;
            origin = lib.mkDefault relyingParty;
            appid = lib.mkDefault relyingParty;
            authfile = "/etc/u2f-mappings";
          };
        };

        environment.etc."u2f-mappings".text = mappings;

        environment.systemPackages = [ pkgs.pam_u2f ];

        forge.runtimeInputs = [ config.systemd.package ];
      };
    };
}
