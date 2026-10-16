# Desktop

The `desktop/` layer turns a host into a graphical machine: a text login screen, a boot splash, and one desktop as the example, Hyprland with a bar, a launcher, a terminal, notifications and a lock screen. Each piece is its own module with its tool's own look, so a fork restyles it in its own config, and another desktop plugs in as a sibling of `hyprland` and reuses the rest.

## Use it

The example host imports it after the `desktop` profile:

```nix
imports = with config.flake.modules.nixos; [
  base
  desktop
  plymouth
  hyprland
  alice
];
```

`hyprland` pulls in `greetd` and `lock`, and gives every home-manager user of the host the Hyprland session through `home-manager.sharedModules`. `plymouth` is independent: import it on any host, with or without a desktop.

## What is in it

- **greetd** with [tuigreet](https://github.com/apognu/tuigreet): clock, hidden password with asterisks, the last user and session remembered (kept in `/persist`, and still valid after a rebuild), every installed Wayland and X session listed. `greetd.command` is the session started when none was picked; `hyprland` sets it to Hyprland through UWSM. Pick `Hyprland (uwsm-managed)` in the session menu, not plain `Hyprland`, which starts without the session services.
- **Plymouth** with the `bgrt` theme (the firmware logo), a quiet kernel and initrd, and the LUKS passphrase asked on the splash.
- **Hyprland** started by [UWSM](https://github.com/Vladimir-csp/uwsm), so the session is a systemd unit tree: `graphical-session.target` is reached, apps run in their own scopes, and user services (waybar, mako, hypridle, the polkit agent) start and stop with the session. The config is written in Hyprland's Lua format.
- **waybar** (workspaces, clock, tray, idle inhibitor, volume, network, battery) in plain text labels, **fuzzel** as launcher, **kitty** as terminal, **mako** for notifications, **cliphist** for clipboard history.
- **Session environment**: home-manager's `home.sessionVariables` (cursor, editor, yours) reach Hyprland and every app through `~/.config/uwsm/env`. XDG user folders (`~/Pictures`, `~/Downloads`, ...) are created.
- **hyprlock** and **hypridle**: locked after 5 minutes idle and before sleep, screen off after 10.
- **Fonts**: NixOS's default font set, and [JetBrainsMono Nerd Font](https://www.nerdfonts.com) as the system monospace font, so kitty and your prompt get its icons. No theme or keyring: colors, GTK and Qt themes, another cursor and a password store are each fork's choice (`gtk`, `qt`, `home.pointerCursor` in home-manager, `services.gnome.gnome-keyring` in NixOS). File pickers, screen sharing and dconf come from what NixOS's `programs.hyprland` turns on (the Hyprland and GTK portals). `NIXOS_OZONE_WL` makes Chromium and Electron apps run on Wayland, and the Adwaita cursor set gives every shape (text, link, resize) to Hyprland and the apps.

## Keybinds

| Keys | Action |
|---|---|
| `Super + Return` | terminal |
| `Super + D` | launcher |
| `Super + Q` | close the window |
| `Super + F` | fullscreen |
| `Super + V` | toggle floating |
| `Super + C` | clipboard history |
| `Super + L` | lock |
| `Super + Shift + E` | leave the session |
| `Super + arrows`, `Super + Shift + arrows` | move focus, swap windows |
| `Super + 1` to `0`, `Super + Shift + 1` to `0` | go to workspace 1 to 10, move the window there |
| `Super + left drag`, `Super + right drag` | move, resize a window |
| `Print`, `Shift + Print` | screenshot of an area, of the screen, saved to `~/Pictures/Screenshots` and copied |
| media and brightness keys | volume, mute, brightness, player, also on the lock screen; volume and brightness repeat while held |

The keyboard layout comes from the host's `services.xserver.xkb`.

## Make it yours

Every value is a default: set the same option in your user's home config to change it.

```nix
programs.kitty.settings.font_size = 12;
programs.kitty.themeFile = "Catppuccin-Mocha";

wayland.windowManager.hyprland.settings = {
  config.general.gaps_out = 4;
  monitor = [
    {
      output = "DP-1";
      mode = "2560x1440@165";
      position = "0x0";
      scale = 1;
    }
    {
      output = "";
      mode = "preferred";
      position = "auto";
      scale = "auto";
    }
  ];
  bind = [
    {
      _args = [
        "SUPER + B"
        (lib.generators.mkLuaInline ''hl.dsp.exec_cmd("uwsm app -- firefox")'')
      ];
    }
  ];
};
```

`bind` is a list: yours is added to the starter's. `monitor` is replaced as a whole: keep the catch-all entry (`output = ""`) for screens you did not list. Colors and fonts are each tool's own (`programs.waybar.style`, `programs.fuzzel.settings.colors`, `services.mako.settings`, `programs.hyprlock.settings`), or one theme for all of them with [Stylix](https://github.com/nix-community/stylix) in your fork. To drop a piece, turn it off (`programs.waybar.enable = false;`). A wallpaper, a night light or an on-screen volume display are left to forks: home-manager has `services.hyprpaper`, `services.hyprsunset` and `services.swayosd`. Start apps from your own binds through `uwsm app --` so they get their own scope.

## Tests

- `desktop-hyprland` logs in through tuigreet and checks the Hyprland socket, `graphical-session.target`, waybar, hypridle, cliphist and the polkit agent, home-manager variables in the session environment, one monitor and no config error, screen off and on, `Super + L` locking and the password unlocking, `Super + Return` opening kitty, and the portal answering.
- `desktop-plymouth` boots a LUKS root with Plymouth and types the passphrase on the splash.
- `<host>-disk` unlocks the installed disk through Plymouth on hosts that import it.

The VM uses a virtio GPU (`-vga none -device virtio-gpu-pci`): Hyprland needs a DRM device the default VGA does not give. `nix build .#nixosConfigurations.<host>.config.system.build.vm` sets it too.
