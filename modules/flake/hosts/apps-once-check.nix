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
  unfree =
    package: pname:
    package.overrideAttrs (old: {
      inherit pname;
      meta = old.meta // {
        license = lib.licenses.unfree;
      };
    });
  overlay = appMachine "apps-overlay" {
    apps = [
      "overlay-hello"
      "clang_12"
      "cowsay"
    ];
    module.nixpkgs.overlays = [
      (_: prev: {
        overlay-hello = unfree prev.hello "overlay-hello";
        clang_12 = unfree prev.hello "revived-alias";
        cowsay = unfree prev.cowsay "renamed-cowsay";
      })
    ];
  } [ ];
  overlayPackages = lib.filter (
    package:
    lib.elem (lib.getName package) [
      "overlay-hello"
      "revived-alias"
      "renamed-cowsay"
    ]
  ) overlay.environment.systemPackages;
  securityKeys = appMachine "apps-once" { features = [ "fido2" ]; } [ ];
  person = lib.head (lib.attrNames config.users);
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
        assert lib.assertMsg
          (lib.length overlayPackages == 3 && lib.all (package: lib.isString package.drvPath) overlayPackages)
          "apps-once: an unfree package of an overlay named in apps (new, re-adding a removed alias, or renaming a nixpkgs package) is not installed";
        assert lib.assertMsg (
          securityKeys.fido2.u2fKeys ? ${person}
        ) "apps-once: the person's u2fKeys do not reach fido2.u2fKeys";
        pkgs.writeText "apps-once" "an app named by the host and the person is imported once, and an unfree package of an overlay works as an app";
    };
}
