{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers) hasDisk hostsOn;

  targetLink = "/run/install-host/disk";

  isoModule =
    target: users:
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      name = target.config.networking.hostName;
      chosenDisk = target.extendModules { modules = [ { disk.device = targetLink; } ]; };

      installHost = pkgs.writeShellApplication {
        name = "install-host";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.util-linux
          pkgs.cryptsetup
          pkgs.jq
          pkgs.mkpasswd
          pkgs.nixos-facter
          config.system.build.nixos-install
        ];
        runtimeEnv = {
          HOST_NAME = name;
          SYSTEM = target.config.system.build.toplevel;
          DISKO = lib.getExe chosenDisk.config.system.build.destroyFormatMount;
          TARGET_LINK = targetLink;
          PASSPHRASE_FILE = target.config.disk.passphraseFile;
          LUKS_NAME = target.config.disko.devices.disk.main.content.partitions.luks.content.name;
          USERS = users;
        };
        text = builtins.readFile ./install-host.sh;
      };
    in
    {
      environment.systemPackages = [ installHost ];

      programs.bash.loginShellInit = ''
        if [ "$(tty)" = /dev/tty1 ] && [ "$(id -u)" -ne 0 ]; then
          install-host || echo "Run install-host to start again."
        fi
      '';

      image.baseName = lib.mkForce "${name}-installer";
    };
in
{
  helpers.installerSystem =
    target: modules:
    let
      host = config.configurations.nixos.${target.config.networking.hostName};
    in
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        "${inputs.nixpkgs}/nixos/modules/installer/cd-dvd/installation-cd-minimal.nix"
        (isoModule target (lib.attrNames host.users))
        { nixpkgs.hostPlatform = host.system; }
      ]
      ++ modules;
    };

  helpers.installerHostsOn = system: lib.filterAttrs (name: _: hasDisk name) (hostsOn system);
}
