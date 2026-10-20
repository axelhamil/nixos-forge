{ lib, config, ... }:

let
  inherit (config.helpers) hostsOn hasDisk testRevision;
in
{
  perSystem =
    { pkgs, system, ... }:
    {
      checks = lib.concatMapAttrs (
        name: host:
        let
          nixos = config.flake.nixosConfigurations.${name}.config;
          tested =
            (config.flake.nixosConfigurations.${name}.extendModules { modules = [ testRevision ]; }).config;
          homeManagerUnits = lib.filter (lib.hasPrefix "home-manager-") (
            lib.attrNames nixos.systemd.services
          );
          hasSdImage = nixos.system.build ? sdImage;
          waitForUnit = unit: "machine.wait_for_unit(${builtins.toJSON unit})";

          withoutDisko =
            { options, ... }:
            {
              imports = [
                host.module
                testRevision
              ];
              config = lib.optionalAttrs (options ? disko) { disko.enableConfig = false; };
            };
        in
        {
          "${name}-toplevel" = nixos.system.build.toplevel;
        }
        // (
          if hasSdImage then
            {
              "${name}-sd-image" = pkgs.runCommand "${name}-sd-image" { } ''
                for image in ${tested.system.build.sdImage}/sd-image/*.img*; do
                  test -s "$image"
                  echo "$(basename "$image") $(stat -c %s "$image")" >> $out
                done
              '';
            }
          else
            {
              "${name}-boots" = pkgs.testers.runNixOSTest {
                name = "${name}-boots";
                node.pkgsReadOnly = false;
                nodes.machine = withoutDisko;
                testScript = lib.concatLines (
                  [ (waitForUnit "multi-user.target") ]
                  ++ map waitForUnit homeManagerUnits
                  ++ [
                    (builtins.readFile ./measure-boot.py)
                    (builtins.readFile ./boot-time.py)
                    host.testScript
                  ]
                );
              };
            }
        )
        // lib.optionalAttrs (hasDisk name) {
          "${name}-disk" = tested.system.build.installTest;
        }
      ) (hostsOn system);
    };
}
