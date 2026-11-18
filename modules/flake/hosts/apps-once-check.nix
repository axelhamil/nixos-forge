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
  evalChecks = lib.mkIf (config.users != { }) {
    apps-once = {
      text = "an app named by the host and the person is imported once, and an unfree package of an overlay works as an app";
      expect = {
        "lock gives no hyprlock PAM service" = twice.security.pam.services ? hyprlock;
        "an app named by the host and the person (zsh, hyprland, lock) changes the system" =
          once.system.build.toplevel.drvPath == twice.system.build.toplevel.drvPath;
        "an unfree package of an overlay named in apps (new, re-adding a removed alias, or renaming a nixpkgs package) is not installed" =
          lib.length overlayPackages == 3 && lib.all (package: lib.isString package.drvPath) overlayPackages;
        "the person's u2fKeys do not reach fido2.u2fKeys" = securityKeys.fido2.u2fKeys ? ${person};
      };
    };
  };
}
