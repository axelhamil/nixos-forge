{ lib, config, ... }:

let
  inherit (config.helpers) appMachine;

  once =
    appMachine "apps-once"
      {
        apps = [
          "hyprland"
          "lock"
        ];
      }
      [
        "zsh"
        "git"
      ];
  twice =
    appMachine "apps-once"
      {
        apps = [
          "zsh"
          "hyprland"
          "lock"
        ];
      }
      [
        "zsh"
        "git"
        "hyprland"
        "lock"
      ];
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux" && config.users != { }) {
      checks.apps-once =
        assert lib.assertMsg (
          twice.security.pam.services ? hyprlock
        ) "apps-once: lock gives no hyprlock PAM service";
        assert lib.assertMsg (
          once.system.build.toplevel.drvPath == twice.system.build.toplevel.drvPath
        ) "apps-once: an app named by the host and the person (zsh, hyprland, lock) changes the system";
        pkgs.writeText "apps-once" "an app named by the host and the person is imported once";
    };
}
