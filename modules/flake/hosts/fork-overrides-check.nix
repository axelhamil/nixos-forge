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
    "services.scx-loader.config.default_sched" = "scx_rusty";
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
  evalChecks = lib.mkIf (config.users != { }) {
    fork-overrides = {
      text = "a fork overrides the shipped policy values with plain definitions, a nixos-hardware profile keeps its NVIDIA choice, and firmware and plymouth evaluate on a Raspberry Pi";
      expect = {
        "a plain definition in the host's module does not take effect: ${lib.concatStringsSep ", " ignored}" =
          ignored == [ ];
        "the nvidia module overrides the open kernel module choice of a nixos-hardware profile" =
          !device.hardware.nvidia.open;
        "firmware and plymouth do not evaluate on a Raspberry Pi" =
          lib.isString pi.system.build.toplevel.drvPath;
      };
    };
  };
}
