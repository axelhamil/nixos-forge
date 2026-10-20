# Desktop

Two features and a handful of apps turn a host into a graphical machine: a text login screen, a boot splash, and Hyprland with a bar, a launcher, a terminal, notifications and a lock screen. This page shows how to list them, what each one brings, the keybinds and how to make it look like yours.

Each piece is its own module with its tool's own look, and you list every one you want: Hyprland without kitty is `kitty` left out of the list. Another desktop plugs in as a sibling of `hyprland`.

## Use it

A host turns it on with the `desktop` profile and these lists:

```nix
profile = "desktop";
hardware = [ "firmware" "audio" "bluetooth" ];
features = [ "greetd" "plymouth" "printing" ];
apps = [ "hyprland" "waybar" "fuzzel" "kitty" "mako" "lock" "upgrade-notify" ];
```

`hyprland` brings only what it cannot run without: uwsm, its portal, a polkit agent, fonts, a cursor and its keybinds. Everything else is listed, and fits in when it is there:

- `greetd` starts Hyprland by default when both are there.
- `kitty` becomes the terminal of `Super + Return`. Any terminal does, through `xdg-terminal-exec`; with none registered, the build warns that the key opens nothing.
- The keys of the launcher, the clipboard history and the lock exist only with `fuzzel` and `lock`, the volume keys only with PipeWire on.
- Named in the host's `apps`, every user of the host gets them. Named in your person's `apps` or in the host's `users.<you>.apps`, only you get their home config, while their NixOS part (Hyprland, fonts, the hyprlock PAM service) is on for the whole machine ([apps.md](apps.md)).
- `plymouth` is independent: import it on any host, with or without a desktop.

The `desktop` profile brings NetworkManager and the performance tuning only ([performance.md](performance.md)): sound, Bluetooth and firmware are listed in `hardware`, printing in `features`.

## What is in it

- **Hyprland** started by [UWSM](https://github.com/Vladimir-csp/uwsm), so user services (waybar, mako, hypridle, the polkit agent) start and stop with the session. The config is written in Hyprland's Lua format.
- **waybar**, **fuzzel** (launcher), **kitty** (terminal), **mako** (notifications), **cliphist** (clipboard history).
- **upgrade-notify**: once a session starts after `auto-upgrade` built a new version, mako shows `System updated` with the package diff summary, once per upgrade, and asks for a restart when it applies at next boot ([updates.md](../explanation/updates.md#know-what-it-did)).
- **hyprlock** and **hypridle**: locked after 5 minutes idle and before sleep, screen off after 10.
- **Session environment**: `home.sessionVariables` reach Hyprland and every app through `~/.config/uwsm/env`. `NIXOS_OZONE_WL` runs Chromium and Electron apps on Wayland.
- **Fonts**: NixOS's default set, and JetBrainsMono Nerd Font as monospace.

No theme, keyring or wallpaper: each is the fork's choice (`gtk`, `qt`, `home.pointerCursor`, `services.hyprpaper` in home-manager, `services.gnome.gnome-keyring` in NixOS).

### `greetd`

The login screen: [tuigreet](https://github.com/apognu/tuigreet) on greetd, with the clock, the password masked, the last user and session remembered (kept in `/persist`), and every installed Wayland and X session listed. `greetd.command` is the session started when none was picked or remembered. Pick `Hyprland (uwsm-managed)`, not plain `Hyprland`, which starts without the session services.

### `plymouth`

The boot splash, with the `bgrt` theme (the firmware logo, a `mkDefault`), a quiet kernel and initrd, and the LUKS passphrase asked on the splash.

## Keybinds

| Keys | Action |
|---|---|
| `Super + Return` | terminal, the default of `xdg-terminal-exec` (kitty when listed) |
| `Super + D` | launcher, with `fuzzel` |
| `Super + Q` | close the window |
| `Super + F` | fullscreen |
| `Super + V` | toggle floating |
| `Super + C` | clipboard history, with `fuzzel` |
| `Super + L` | lock, with `lock` |
| `Super + Shift + E` | leave the session |
| `Super + arrows`, `Super + Shift + arrows` | move focus, swap windows |
| `Super + 1` to `0`, `Super + Shift + 1` to `0` | go to workspace 1 to 10, move the window there |
| `Super + left drag`, `Super + right drag` | move, resize a window |
| `Print`, `Shift + Print` | screenshot of an area, of the screen, saved to `~/Pictures/Screenshots` and copied |
| media and brightness keys | volume and mute (with PipeWire, which nixpkgs turns on for a graphical session), brightness, player, also on the lock screen; volume and brightness repeat while held |

The keyboard layout comes from the host's `keyboard` field (`services.xserver.xkb`).

## Make it yours

Every value is a default: set the same option in your person's `module` to change it ([apps.md](apps.md#change-a-starter-default)).

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

`bind` is a list: yours is added to the app's. `monitor` is replaced as a whole: keep the catch-all entry (`output = ""`) for screens you did not list. Colors and fonts are each tool's own (`programs.waybar.style`, `programs.fuzzel.settings.colors`, `services.mako.settings`, `programs.hyprlock.settings`), or one theme for all of them with [Stylix](https://github.com/nix-community/stylix) in your fork. To drop a piece, turn it off (`programs.waybar.enable = false;`). Start apps from your own binds through `uwsm app --` so they get their own scope.

## Apps

An app with its config lives in `modules/apps/<name>` and is named in a host's or user's `apps` (any other name resolves to a nixpkgs package). [`modules/apps/firefox`](../../modules/apps/firefox/default.nix) is the example: telemetry, studies and sponsored content off, tracking protection on, every policy a default you override, and the profile persisted with `wipe-home`. Copy it for another browser or app.

## VM

`nix build .#nixosConfigurations.<host>.config.system.build.vm` uses a virtio GPU (`-vga none -device virtio-gpu-pci`), since Hyprland needs a DRM device. VM tests are listed in [ci.md](../how-to/ci.md).
