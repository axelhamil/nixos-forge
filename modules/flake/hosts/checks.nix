{ lib, config, ... }:

let
  inherit (config.helpers) hostsOn;
in
{
  perSystem =
    { pkgs, system, ... }:
    {
      checks = lib.concatMapAttrs (
        name: host:
        let
          nixos = config.flake.nixosConfigurations.${name}.config;
          homeManagerUnits = lib.filter (lib.hasPrefix "home-manager-") (
            lib.attrNames nixos.systemd.services
          );
          hasSdImage = nixos.system.build ? sdImage;
          waitForUnit = unit: "machine.wait_for_unit(${builtins.toJSON unit})";

          withoutDisko =
            { options, ... }:
            {
              imports = [ host.module ];
              config = lib.optionalAttrs (options ? disko) { disko.enableConfig = false; };
            };
        in
        (
          if hasSdImage then
            { "${name}-sd-image" = nixos.system.build.sdImage; }
          else
            {
              "${name}-toplevel" = nixos.system.build.toplevel;

              "${name}-boots" = pkgs.testers.runNixOSTest {
                name = "${name}-boots";
                node.pkgsReadOnly = false;
                nodes.machine = withoutDisko;
                testScript = lib.concatLines (
                  [ (waitForUnit "multi-user.target") ] ++ map waitForUnit homeManagerUnits ++ [ host.testScript ]
                );
              };
            }
        )
        // lib.optionalAttrs (nixos.system.build ? installTest) {
          "${name}-disk" = nixos.system.build.installTest;
        }
      ) (hostsOn system);
    };
}
