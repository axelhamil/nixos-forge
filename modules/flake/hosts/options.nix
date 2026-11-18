{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (lib) mkOption types;
  inherit (config) base profiles;
  resolve = import ./_resolve.nix { inherit lib inputs config; };
  inherit (resolve) pick selection;
  inherit (import ./_account.nix { inherit lib config resolve; }) accountOptions userModule;
  fields = import ./_fields.nix { inherit lib config accountOptions; };

  hostOptions =
    { name, config, ... }:
    let
      host = config;
    in
    {
      options = fields;

      config.module =
        { lib, ... }:
        {
          imports = [
            base
            (selection name host.system "" host)
          ]
          ++ lib.optional (host.profile != null) (pick name "profile" profiles host.profile)
          ++ lib.mapAttrsToList (userModule name host.apps host.system) host.users;

          specialisation = lib.mapAttrs (variant: chosen: {
            configuration = {
              imports = [
                (selection name host.system "specialisations.${variant}." chosen)
                chosen.module
              ];
              environment.etc.specialisation.text = variant;
            };
          }) host.specialisations;

          networking.hostName = name;
          update = { inherit (host) flake mirrors signers; };
          hardware.facter.reportPath = lib.mkIf (host.facter != null) host.facter;
          time.timeZone = lib.mkDefault host.timeZone;
          i18n.defaultLocale = lib.mkDefault host.locale;
          services.xserver.xkb.layout = lib.mkDefault host.keyboard;
          console.keyMap = lib.mkDefault host.keyboard;

          system.stateVersion = host.stateVersion;
        };
    };
in
{
  options.configurations.nixos = mkOption {
    type = types.lazyAttrsOf (types.submodule hostOptions);
    default = { };
    description = "Machines of the flake, one file each under `modules/hosts/<name>`. Each one gets a nixosConfigurations entry and its checks.";
  };
}
