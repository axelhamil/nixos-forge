{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config.helpers)
    flakePackage
    hasDisk
    hostsOn
    wrapForge
    ;

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

      installConfig = pkgs.writeText "forge-install.json" (
        builtins.toJSON {
          hostName = name;
          system = "${target.config.system.build.toplevel}";
          disko = lib.getExe chosenDisk.config.system.build.destroyFormatMount;
          besideWindows = lib.getExe target.config.system.build.installBesideWindows;
          dualBoot = target.options ? dualBoot;
          inherit targetLink users;
          passphraseFile = target.config.disk.passphraseFile;
          luksName = target.config.disko.devices.disk.main.content.partitions.luks.content.name;
        }
      );

      unwrapped = flakePackage pkgs "forge";

      forge = wrapForge pkgs unwrapped {
        runtimeInputs = [
          pkgs.coreutils
          pkgs.util-linux
          pkgs.cryptsetup
          pkgs.mkpasswd
          pkgs.nixos-facter
          config.system.build.nixos-install
        ];
        environment.FORGE_INSTALL_CONFIG = "${installConfig}";
      };
    in
    {
      environment.systemPackages = [ forge ];

      programs.bash.loginShellInit = ''
        if [ "$(tty)" = /dev/tty1 ] && [ "$(id -u)" -ne 0 ]; then
          forge install || echo "Run forge install to start again."
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
