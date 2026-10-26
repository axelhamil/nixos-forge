# Start `/home` empty at every boot

A host with `disk` that adds `wipe-home` to its features forgets `/home` like `/`: every boot starts from an empty home, and only the paths you declared come back. A fresh home at every boot shows you which files you actually use.

> [!WARNING]
> Anything in a home that you did not declare is gone at the next boot, kept only in a snapshot that a later boot deletes after 7 days, unless it is one of the 2 most recent. On a home in use, follow [Turn it on for a home in use](#turn-it-on-for-a-home-in-use).

This page shows what happens at boot, how to declare what you keep, how to find what you forgot, and how to switch a home already in use.

```nix
features = [ "disk" "wipe-home" ];
```

## What happens at boot

- The initrd moves the previous `@home` to `/.snapshots/home-wiped.<UTC date>`, read-only, and creates an empty one.
- A wiped home is deleted by a later boot after 7 days; the 2 most recent always stay. Copy a forgotten file back from there.
- btrbk stops snapshotting `/home` and keeps snapshotting `/persist`, where declared paths live ([services.md](../reference/services.md#snapshots-btrbk)).
- home-manager links its files again, so your config, shell and tools are there as usual. `~/.cache` starts empty, so the first launch of some tools and the first `nh os switch` after a boot are slower.

## Declare what to keep

Declared paths live in `/persist/home/<you>` and are bind-mounted into your home at boot. The shipped modules already declare their own state when `wipe-home` is on:

| Declared by | Path |
|---|---|
| `base` | `~/.ssh` and `~/.gnupg`, mode `0700`, in every home |
| `gh` | `~/.config/gh`, mode `0700` |
| `forge update` | the host's `flake` checkout, when it is a path in `/home/<you>/` (do not list it again) |
| `zsh` | `~/.local/state/zsh` |
| `zoxide` | `~/.local/share/zoxide` |
| `neovim` | `~/.local/share/nvim`, `~/.local/state/nvim` |
| `stow` | the clone (`~/dotfiles`) and `~/.local/state/stow` |
| `direnv` | `~/.local/share/direnv` |
| `docker` | `~/.local/share/docker` |
| `audio` | `~/.local/state/wireplumber` |
| `firefox` | the Firefox profile |
| `hyprland` | `~/Pictures/Screenshots`, under the pictures directory (`xdg.userDirs.pictures`) |
| `steam` | `~/.steam` and `~/.local/share/Steam` |
| `mangohud` | `~/.local/state/mangohud`, its logs |
| `upgrade-notify` | `~/.local/state/upgrade-notify` |
| `wipe-home` | `~/.local/share/systemd/timers` (missed user timers still catch up) |

Declare yours in your person's `module` ([apps.md](../reference/apps.md#change-a-default)), relative to your home:

```nix
users.<you>.module = {
  home.persistence."/persist" = {
    directories = [
      "projects"
      "Documents"
      { directory = ".local/share/keyrings"; mode = "0700"; }
      ".local/share/nix"
    ];
    files = [ ".config/monitors.xml" ];
  };
};
```

- Prefer directories to files: a persisted file is a bind mount, so an app that saves by renaming a new file over it fails.
- `mode` only applies when the directory is first created in `/persist`.
- Without `wipe-home` these lines do nothing, so the same user works on hosts that keep their home.
- Packages installed with `nix profile add` are lost at reboot: name them in your `apps` ([apps.md](../reference/apps.md)).

## Find what is missing

Usual candidates: projects and documents, other browser profiles (`~/.config/chromium`), keyrings (`~/.local/share/keyrings`), app sessions under `~/.config` and `~/.local/share`, and `~/.local/share/nix` for the flake settings you trusted.

On a home you are about to wipe, list what changed in the last 30 days:

```sh
find ~ -xdev -type f -mtime -30 -not -path '*/.cache/*' -printf '%P\n' | cut -d/ -f1-2 | sort | uniq -c | sort -rn | head -50
```

Once `wipe-home` is on, the last wiped home shows what is not declared yet:

```sh
ls /.snapshots
find /.snapshots/home-wiped.<date>/<you> -xdev -type f -printf '%P\n' | cut -d/ -f1-2 | sort | uniq -c | sort -rn
```

## Turn it on for a home in use

1. Declare your paths, then apply at the next boot rather than live: `nh os boot` (or `nixos-rebuild boot`), and reboot.
2. Check that the home is now wiped and your paths are mounted from `/persist`:

   ```sh
   ls /.snapshots                # a home-wiped.<date> folder: the home you had
   findmnt ~/projects            # a path you declared: SOURCE ends in [/@persist/home/<you>/projects]
   ```

3. Log in on a console, before a browser or another app writes fresh state, and copy every declared path back from `/.snapshots/home-wiped.<date>` within 7 days. The loop below reads the declared paths from the mounts of `/persist` in your home, so it covers those of the shipped modules too, paths with spaces included. What it copies lands in `/persist`:

   ```sh
   systemctl --user stop docker
   old=/.snapshots/home-wiped.<date>/$USER
   findmnt -rn -o TARGET,SOURCE | while read -r target device; do
     case $device in *"[/@persist/"*) ;; *) continue ;; esac
     target=$(printf '%b' "$target")
     case $target in "$HOME"/*) ;; *) continue ;; esac
     from="$old/${target#"$HOME"/}"
     if [ -d "$from" ]; then cp -a --reflink=auto "$from/." "$target/"
     elif [ -e "$from" ]; then cp -a --reflink=auto "$from" "$target"
     fi
   done
   sudo systemctl restart home-manager-$USER
   ```

   Restarting home-manager links your dotfiles, which `stow` found missing at boot.

Never create `/persist/home/<you>` by hand as root: impermanence copies its owner onto your home directory.
