{ config, ... }:

let
  nixosModules = config.flake.modules.nixos;
in
{
  perSystem =
    { pkgs, ... }:
    {
      checks.desktop-boots = pkgs.testers.runNixOSTest {
        name = "desktop-boots";

        nodes.machine.imports = with nixosModules; [
          base
          axel
          desktop
        ];

        testScript = ''
          machine.wait_for_unit("multi-user.target")
          machine.wait_for_unit("home-manager-axel.service")
          machine.succeed("id axel")
        '';
      };
    };
}
