{
  lib,
  inputs,
  options,
  ...
}:

let
  hostType = options.configurations.nixos.type.nestedTypes.elemType;

  machine =
    hostApps: userApps:
    let
      host = lib.evalModules {
        modules = hostType.getSubModules ++ [
          {
            _module.args.name = "apps-once";
            stateVersion = "26.05";
            apps = hostApps;
            users.tester.apps = userApps;
            module = {
              fileSystems."/" = {
                device = "/dev/disk/by-label/apps-once";
                fsType = "ext4";
              };
              boot.loader.grub.enable = false;
            };
          }
        ];
      };
    in
    (inputs.nixpkgs.lib.nixosSystem {
      modules = [
        host.config.module
        { nixpkgs.hostPlatform = "x86_64-linux"; }
      ];
    }).config;

  once = machine [ "hyprland" ] [ "shell" ];
  twice = machine [ "shell" "zsh" "hyprland" "lock" ] [ "shell" "zsh" "git" ];

  hyprlockServices = lib.filter (name: name == "hyprlock") (
    lib.attrNames twice.security.pam.services
  );
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks.apps-once =
        assert lib.assertMsg (
          lib.length hyprlockServices == 1
        ) "apps-once: expected one hyprlock pam service, got ${toString (lib.length hyprlockServices)}";
        assert lib.assertMsg (
          once.system.build.toplevel.drvPath == twice.system.build.toplevel.drvPath
        ) "apps-once: an app reached twice (shell and zsh, hyprland and lock) changes the system";
        pkgs.writeText "apps-once" "an app reached twice is imported once";
    };
}
