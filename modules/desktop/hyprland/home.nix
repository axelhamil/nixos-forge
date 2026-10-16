{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
  inherit (config.flake.modules) homeManager;
in
{
  flake.modules.homeManager.hyprland =
    {
      lib,
      pkgs,
      config,
      osConfig,
      ...
    }:
    let
      inherit (config.theme) palette;
      inherit (osConfig.services.xserver) xkb;

      bindWith = flags: key: dispatcher: {
        _args = [
          key
          (lib.generators.mkLuaInline dispatcher)
        ]
        ++ lib.optional (flags != { }) flags;
      };
      bind = bindWith { };
      bindLocked = bindWith { locked = true; };
      bindRepeating = bindWith {
        locked = true;
        repeating = true;
      };
      bindMouse = bindWith { mouse = true; };

      exec = command: "hl.dsp.exec_cmd(${builtins.toJSON command})";
      run = command: exec "uwsm app -- ${command}";
      rgb = color: "rgb(${color})";

      clipboardHistory = pkgs.writeShellApplication {
        name = "clipboard-history";
        runtimeInputs = with pkgs; [
          cliphist
          fuzzel
          wl-clipboard
        ];
        text = "cliphist list | fuzzel --dmenu | cliphist decode | wl-copy";
      };

      screenshot = pkgs.writeShellApplication {
        name = "screenshot";
        runtimeInputs = with pkgs; [
          grim
          slurp
          wl-clipboard
          libnotify
          coreutils
        ];
        text = builtins.readFile ./screenshot.sh;
      };

      workspaces = lib.concatMap (
        number:
        let
          key = toString (lib.mod number 10);
        in
        [
          (bind "SUPER + ${key}" "hl.dsp.focus({ workspace = ${toString number} })")
          (bind "SUPER + SHIFT + ${key}" "hl.dsp.window.move({ workspace = ${toString number} })")
        ]
      ) (lib.range 1 10);

      directions =
        lib.concatMap
          (direction: [
            (bind "SUPER + ${direction}" "hl.dsp.focus({ direction = \"${direction}\" })")
            (bind "SUPER + SHIFT + ${direction}" "hl.dsp.window.swap({ direction = \"${direction}\" })")
          ])
          [
            "left"
            "right"
            "up"
            "down"
          ];
    in
    {
      imports = with homeManager; [
        theme
        waybar
        fuzzel
        ghostty
        mako
        lock
      ];

      home.packages = with pkgs; [
        screenshot
        clipboardHistory
        wl-clipboard
        brightnessctl
        playerctl
      ];

      services.hyprpolkitagent.enable = lib.mkDefault true;
      services.cliphist.enable = lib.mkDefault true;

      xdg.userDirs = mkDefaults {
        enable = true;
        createDirectories = true;
      };

      xdg.configFile."uwsm/env".source =
        "${config.home.sessionVariablesPackage}/etc/profile.d/hm-session-vars.sh";

      wayland.windowManager.hyprland = {
        enable = true;
        package = null;
        portalPackage = null;
        systemd.enable = false;
        configType = "lua";

        settings = {
          monitor = lib.mkDefault [
            {
              output = "";
              mode = "preferred";
              position = "auto";
              scale = "auto";
            }
          ];

          config = mkDefaults {
            general = {
              gaps_in = 4;
              gaps_out = 8;
              border_size = 2;
              col = {
                active_border = rgb palette.accent;
                inactive_border = rgb palette.overlay;
              };
              layout = "dwindle";
            };
            decoration = {
              rounding = 8;
              shadow.enabled = false;
              blur.enabled = false;
            };
            animations.enabled = true;
            dwindle.preserve_split = true;
            input = {
              kb_layout = xkb.layout;
              kb_variant = xkb.variant;
              kb_options = xkb.options;
              follow_mouse = 1;
              touchpad.natural_scroll = true;
            };
            misc = {
              disable_hyprland_logo = true;
              force_default_wallpaper = 0;
              background_color = rgb palette.background;
            };
            ecosystem.no_update_news = true;
          };

          bind = [
            (bind "SUPER + Return" (run "ghostty"))
            (bind "SUPER + D" (run "fuzzel"))
            (bind "SUPER + Q" "hl.dsp.window.close()")
            (bind "SUPER + F" "hl.dsp.window.fullscreen()")
            (bind "SUPER + V" "hl.dsp.window.float({ action = \"toggle\" })")
            (bind "SUPER + C" (run "clipboard-history"))
            (bind "SUPER + L" (exec "loginctl lock-session"))
            (bind "SUPER + SHIFT + E" (exec "uwsm stop"))
            (bind "Print" (exec "screenshot area"))
            (bind "SHIFT + Print" (exec "screenshot screen"))
            (bindMouse "SUPER + mouse:272" "hl.dsp.window.drag()")
            (bindMouse "SUPER + mouse:273" "hl.dsp.window.resize()")
            (bindRepeating "XF86AudioRaiseVolume" (exec "wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"))
            (bindRepeating "XF86AudioLowerVolume" (exec "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"))
            (bindRepeating "XF86MonBrightnessUp" (exec "brightnessctl set 5%+"))
            (bindRepeating "XF86MonBrightnessDown" (exec "brightnessctl set 5%-"))
            (bindLocked "XF86AudioMute" (exec "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"))
            (bindLocked "XF86AudioMicMute" (exec "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"))
            (bindLocked "XF86AudioPlay" (exec "playerctl play-pause"))
            (bindLocked "XF86AudioNext" (exec "playerctl next"))
            (bindLocked "XF86AudioPrev" (exec "playerctl previous"))
          ]
          ++ directions
          ++ workspaces;
        };
      };
    };
}
