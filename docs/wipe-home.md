# Start `/home` empty at every boot

A host with `disk` that adds `wipe-home` to its features forgets `/home` like `/`: every boot starts from an empty home, and only the paths you declared come back.

```nix
features = [ "disk" "wipe-home" ];
```

## What happens at boot

- The initrd moves the previous `@home` to `/.snapshots/home-wiped.<UTC date>`, read-only, and creates an empty one.
- A wiped home is deleted by a later boot after 7 days; the 2 most recent always stay. Copy a forgotten file back from there.
- btrbk stops snapshotting `/home` and keeps snapshotting `/persist`, where declared paths live ([services.md](services.md#snapshots-btrbk)).
- home-manager links its files again, so your config, shell and tools are there as usual. `~/.cache` starts empty, so the first launch of some tools and the first `nh os switch` after a boot are slower.

## Declare what to keep

Declared paths live in `/persist/home/<you>` and are bind-mounted into your home at boot. The starter's modules already declare their own state when `wipe-home` is on:

| Module | Path |
|---|---|
| `zsh` | `~/.local/state/zsh` |
| `cli` | `~/.local/share/zoxide` |
| `neovim` | `~/.local/share/nvim`, `~/.local/state/nvim` |
| `stow` | the clone (`~/dotfiles`) and `~/.local/state/stow` |
| `direnv` | `~/.local/share/direnv` |
| `docker` | `~/.local/share/docker` |
| `audio` | `~/.local/state/wireplumber` |
| `firefox` | the Firefox profile |
| `wipe-home` | `~/.local/share/systemd/timers` (missed user timers still catch up) |

Declare yours in your user's home config, relative to your home:

```nix
flake.modules.homeManager.<you> = {
  home.persistence."/persist" = {
    directories = [
      "projects"
      "Documents"
      { directory = ".ssh"; mode = "0700"; }
      { directory = ".gnupg"; mode = "0700"; }
      ".local/share/nix"
    ];
    files = [ ".config/monitors.xml" ];
  };
};
```

- Prefer directories to files: a persisted file is a bind mount, so an app that saves by renaming a new file over it fails.
- `mode` only applies when the directory is first created in `/persist`.
- Without `wipe-home` these lines do nothing, so the same user works on hosts that keep their home.
- Packages installed with `nix profile add` are lost at reboot: list them in your home config.

## Find what is missing

Usual candidates: projects and documents, `~/.ssh`, `~/.gnupg`, other browser profiles (`~/.config/chromium`), keyrings (`~/.local/share/keyrings`), app sessions under `~/.config` and `~/.local/share`, and `~/.local/share/nix` for the flake settings you trusted.

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
2. Log in on a console, before a browser or another app writes fresh state, and copy each declared path back from `/.snapshots/home-wiped.<date>` within 7 days, the starter's own included. It lands in `/persist`:

   ```sh
   systemctl --user stop docker
   old=/.snapshots/home-wiped.<date>/$USER
   for path in projects dotfiles .local/state/zsh .local/share/zoxide .local/share/direnv .local/share/docker; do
     [ -e "$old/$path" ] && cp -a --reflink=auto "$old/$path/." ~/"$path/"
   done
   sudo systemctl restart home-manager-$USER
   ```

   Restarting home-manager links your dotfiles, which `stow` found missing at boot.

Never create `/persist/home/<you>` by hand as root: impermanence copies its owner onto your home directory.
