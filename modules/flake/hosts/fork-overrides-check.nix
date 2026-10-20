{ lib, config, ... }:

let
  inherit (config.helpers) appMachine;

  overrides = {
    "hardware.nvidia.open" = false;
    "hardware.nvidia.powerManagement.enable" = false;
    "hardware.bluetooth.powerOnBoot" = false;
    "boot.loader.systemd-boot.bootCounting.tries" = 5;
    "boot.lanzaboote.bootCounting.initialTries" = 5;
    "services.btrbk.instances.snapshots.onCalendar" = "daily";
    "services.btrbk.instances.snapshots.settings.snapshot_preserve" = "7d";
    "services.openssh.settings.PermitRootLogin" = "prohibit-password";
    "services.scx.scheduler" = "scx_rusty";
    "systemd.timers.auto-upgrade.timerConfig.OnCalendar" = "daily";
    "virtualisation.docker.rootless.daemon.settings.log-driver" = "journald";
    "services.greetd.settings.default_session.command" = "agreety --cmd sway";
    "greetd.command" = "sway";
    "environment.sessionVariables.NIXOS_OZONE_WL" = "0";
    "programs.zsh.enableGlobalCompInit" = true;
    "programs.steam.protontricks.enable" = false;
    "programs.nix-index-database.comma.enable" = false;
  };

  pathOf = lib.splitString ".";

  forked = appMachine "fork-overrides" {
    profile = "desktop";
    features = [
      "disk"
      "secure-boot"
      "ssh"
      "auto-upgrade"
      "docker"
      "greetd"
    ];
    hardware = [
      "nvidia"
      "bluetooth"
    ];
    apps = [
      "hyprland"
      "zsh"
      "steam"
      "nix-index"
    ];
    module = lib.foldl' lib.recursiveUpdate { } (
      lib.mapAttrsToList (name: lib.setAttrByPath (pathOf name)) overrides
    );
  } [ ];

  ignored = lib.attrNames (
    lib.filterAttrs (name: value: lib.attrByPath (pathOf name) null forked != value) overrides
  );

  device = appMachine "fork-overrides-device" {
    hardware = [
      "nvidia"
      "asus-fa507nv"
    ];
  } [ ];

  pi = appMachine "fork-overrides-pi" {
    system = "aarch64-linux";
    profile = "pi";
    features = [ "plymouth" ];
    hardware = [
      "raspberry-pi-5"
      "firmware"
    ];
  } [ ];
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux" && config.users != { }) {
      checks.fork-overrides =
        assert lib.assertMsg (ignored == [ ])
          "fork-overrides: a plain definition in the host's module does not take effect: ${lib.concatStringsSep ", " ignored}";
        assert lib.assertMsg (!device.hardware.nvidia.open)
          "fork-overrides: the nvidia module overrides the open kernel module choice of a nixos-hardware profile";
        builtins.seq pi.system.build.toplevel.drvPath (
          pkgs.writeText "fork-overrides" "a fork overrides the shipped policy values with plain definitions, a nixos-hardware profile keeps its NVIDIA choice, and firmware and plymouth evaluate on a Raspberry Pi"
        );
    };
}
