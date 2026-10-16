{ lib, config, ... }:

let
  inherit (config.helpers) installerSystem installerHostsOn hasDisk;
  inherit (config.installer) testedHosts;

  untestable = lib.filter (
    name: !(config.configurations.nixos ? ${name}) || !hasDisk name
  ) testedHosts;

  testing =
    { modulesPath, ... }:
    {
      imports = [
        "${modulesPath}/testing/test-instrumentation.nix"
        "${modulesPath}/profiles/qemu-guest.nix"
      ];
    };
in
{
  perSystem =
    { pkgs, system, ... }:
    let
      qemu = import "${pkgs.path}/nixos/lib/qemu-common.nix" {
        inherit lib;
        inherit (pkgs) stdenv;
      };

      installerCheck =
        name:
        let
          nixos = config.flake.nixosConfigurations.${name};
          target = nixos.extendModules { modules = [ testing ]; };
          iso = installerSystem target [
            testing
            {
              isoImage.squashfsCompression = "zstd -Xcompression-level 3";
              users.users.root.initialHashedPassword = lib.mkForce null;
            }
          ];
        in
        pkgs.testers.runNixOSTest {
          name = "${name}-installer";
          nodes = { };
          testScript = ''
            qemu = "${qemu.qemuBinary pkgs.qemu_test}"
            qemu_img = "${pkgs.qemu_test}/bin/qemu-img"
            firmware = "${pkgs.OVMF.firmware}"
            firmware_vars = "${pkgs.OVMF.variables}"
            iso = "${iso.config.system.build.isoImage}/iso/${iso.config.image.fileName}"
            host_name = ${builtins.toJSON name}
            users = ${builtins.toJSON (lib.attrNames config.configurations.nixos.${name}.users)}
            passphrase_file = ${builtins.toJSON nixos.config.disk.passphraseFile}

            ${nixos.config.disk.unlockTestScript}
            ${builtins.readFile ./installer-test.py}
            ${builtins.readFile ./installed-test.py}
          '';
        };
    in
    {
      checks =
        lib.throwIf (untestable != [ ])
          "installer.testedHosts: ${lib.concatStringsSep ", " untestable} has no installer: a tested host must exist and import disk."
          (
            lib.mapAttrs' (name: _: lib.nameValuePair "${name}-installer" (installerCheck name)) (
              lib.filterAttrs (name: _: lib.elem name testedHosts) (installerHostsOn system)
            )
          );
    };
}
