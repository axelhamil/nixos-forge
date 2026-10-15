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
          waitForUnit = unit: "machine.wait_for_unit(${builtins.toJSON unit})";

          withoutDisko =
            { options, ... }:
            {
              imports = [ host.module ];
              config = lib.optionalAttrs (options ? disko) { disko.enableConfig = false; };
            };
        in
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
        // lib.optionalAttrs (nixos.system.build ? installTest) {
          "${name}-disk" = nixos.system.build.installTest;
        }
      ) (hostsOn system);
    };
}
