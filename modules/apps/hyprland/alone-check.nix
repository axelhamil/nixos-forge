{ lib, config, ... }:

let
  inherit (config.helpers) appMachine;

  person = lib.head (lib.attrNames config.users);

  machineOf = host: appMachine "hyprland-alone" host [ ];

  homeOf = machine: machine.home-manager.users.${person};

  bindsOf = machine: builtins.toJSON (homeOf machine).wayland.windowManager.hyprland.settings.bind;

  warningsOf = machine: (homeOf machine).warnings;

  hasTerminalWarning = machine: lib.any (lib.hasInfix "no default terminal") (warningsOf machine);

  alone = machineOf { apps = [ "hyprland" ]; };
  silent = machineOf {
    apps = [ "hyprland" ];
    module.services.pipewire.enable = false;
  };
  desktop = machineOf {
    apps = [
      "hyprland"
      "kitty"
      "fuzzel"
      "lock"
    ];
    hardware = [ "audio" ];
  };

  bindsNeeding = machine: {
    fuzzel = (homeOf machine).programs.fuzzel.enable;
    clipboard-history = (homeOf machine).programs.fuzzel.enable;
    lock-session = (homeOf machine).programs.hyprlock.enable;
    wpctl = machine.services.pipewire.wireplumber.enable;
  };

  strayBinds =
    machine:
    lib.attrNames (
      lib.filterAttrs (word: enabled: !enabled && lib.hasInfix word (bindsOf machine)) (
        bindsNeeding machine
      )
    );

  missingBinds = lib.filter (word: !lib.hasInfix word (bindsOf desktop)) (
    lib.attrNames (bindsNeeding desktop)
  );
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux" && config.users != { }) {
      checks.hyprland-alone =
        assert lib.assertMsg (strayBinds alone == [ ])
          "hyprland-alone: hyprland alone binds a key to a program it lacks: ${toString (strayBinds alone)}";
        assert lib.assertMsg (strayBinds silent == [ ] && !lib.hasInfix "wpctl" (bindsOf silent))
          "hyprland-alone: hyprland without PipeWire binds a key to a program it lacks: ${toString (strayBinds silent)}";
        assert lib.assertMsg (
          !lib.hasInfix "kitty" (bindsOf alone) && lib.hasInfix "xdg-terminal-exec" (bindsOf alone)
        ) "hyprland-alone: the terminal key does not go through xdg-terminal-exec";
        assert lib.assertMsg (homeOf alone).xdg.terminal-exec.enable
          "hyprland-alone: xdg-terminal-exec is not installed";
        assert lib.assertMsg (hasTerminalWarning alone)
          "hyprland-alone: hyprland alone does not warn that Super+Return opens nothing";
        assert lib.assertMsg (missingBinds == [ ])
          "hyprland-alone: with fuzzel, lock and audio, hyprland lacks the binds for ${toString missingBinds}";
        assert lib.assertMsg (
          !hasTerminalWarning desktop
        ) "hyprland-alone: with kitty, hyprland still warns that no terminal is registered";
        assert lib.assertMsg (lib.hasInfix "kitty.desktop"
          (homeOf desktop).xdg.configFile."xdg-terminals.list".text
        ) "hyprland-alone: kitty is not the default terminal of xdg-terminal-exec";
        pkgs.writeText "hyprland-alone" "hyprland works alone, warns without a terminal and binds only the apps and audio it has";
    };
}
