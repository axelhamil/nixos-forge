# Shell and tools

The `shell` bundle gives a user a ready shell with no choice to make: zsh, starship, git, neovim and a set of CLI tools, all through home-manager. Each tool is its own module under `modules/home/`, so you can drop or swap one.

## Use it

Every user gets it: `home` defaults to `[ "shell" ]` in the host file, and makes zsh the login shell.

```nix
users.<you>.home = [ "shell" ];
```

To pick tools one by one, list them instead of `shell`: `zsh`, `starship`, `git`, `neovim`, `cli`. Every value is a default: set the same option in your user's home config (a `flake.modules.homeManager.<you>` in `modules/users/<you>/default.nix`, listed in your `home` next to `shell`) to change it, `false` included.

## What is in it

- **zsh** with autosuggestions, syntax highlighting and a shared, deduplicated history of 100,000 lines. The NixOS `zsh` module makes it a login shell, completes system commands and skips the global `compinit`, so completion is initialised once, by home-manager, instead of twice. That switch is system-wide: another zsh user of the host needs home-manager's zsh too (`zsh` or `shell` in its `home`), or gets no completion.
- **starship** prompt with plain text symbols, readable on a console without a Nerd Font. Once you have one, drop the preset: `programs.starship.presets = [ ];`.
- **git** with the settings git's own developers use: `main` as first branch, recent branches first, `push.autoSetupRemote`, pruned fetches, histogram diffs with moved lines highlighted, the diff shown while writing a commit, `zdiff3` conflicts, `rerere`, rebases that autostash and update stacked branches, autocorrect on prompt. Diffs and blame page through [delta](https://github.com/dandavison/delta) (`n` and `N` jump between files). Set your identity in your home config:

  ```nix
  programs.git.settings.user = {
    name = "Your Name";
    email = "you@example.org";
  };
  ```

  To sign commits with your SSH key:

  ```nix
  programs.git.signing = {
    format = "ssh";
    key = "~/.ssh/id_ed25519.pub";
    signByDefault = true;
  };
  ```

- **neovim** as `$EDITOR` (`vi` and `vim` too), with the compiler, tree-sitter, ripgrep and fd that most configurations expect. The starter does not manage its configuration: put your own in `~/.config/nvim`, [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim) or [LazyVim](https://www.lazyvim.org) for instance. Language servers that a plugin downloads (Mason) need the `nix-ld` module ([dev.md](dev.md)).
- **cli**: fzf (`Ctrl-R` history, `Ctrl-T` files and `Alt-C` directories, listed by fd so `.gitignore` is respected), zoxide (`z <part of a path>`), eza with git status (`ll`, `la`, `lt`; `ls` stays `ls`, whose flags eza does not all accept), bat, ripgrep, fd, btop and jq.

Files follow the XDG layout: zsh lives in `~/.config/zsh`, its history in `~/.local/state/zsh`, Nix profiles in `~/.local/state/nix`. The only file the shell leaves in your home is `~/.zshenv`, which tells zsh where its configuration is. On a home that predates it, move `~/.zsh_history` to `~/.local/state/zsh/history` to keep your history; old `~/.nix-profile` and `~/.nix-defexpr` links are stale and can go.

The terminal emulator and its font come with the desktop.

`home-shell` logs in a user with the bundle and checks zsh is the login shell, `compinit` runs once, starship loads, git uses `zdiff3` and delta, neovim is the editor and starts without a managed configuration, and every CLI tool is on the path while `ls` is left alone, and that nothing lands outside the XDG directories.

## Bring your dotfiles

Add `stow` next to `shell`, clone your dotfiles repository to `~/dotfiles`, and every switch links it into your home with [GNU stow](https://www.gnu.org/software/stow/). Edits in the repository are live; a new file is linked at the next switch, or right away with `systemctl restart home-manager-$USER`.

```nix
users.<you>.home = [ "shell" "stow" ];
```

Both common layouts work with no setting:

- **A mirror of your home**: `~/dotfiles/.zshrc`, `~/dotfiles/.config/nvim/init.lua`. Recognised when the repository root holds dot files; its `.git`, `README` and `LICENSE` are not linked.
- **Stow packages**: one directory per tool, `~/dotfiles/nvim/.config/nvim/init.lua`. Every top-level directory is linked, or only those you list.

Options:

- `stow.directory`: where the clone is, `~/dotfiles` by default.
- `stow.packages`: the packages to link, for instance `[ "nvim" "tmux" ]`; empty by default (everything).
- `stow.dotfiles`: stow's `--dotfiles`, for `dot-config` style names; off by default.

What to expect:

- Directories stay real, only files are linked (`--no-folding`), so home-manager and stow share `~/.config`.
- Without the repository nothing happens: a fresh install boots, then you clone and restart the unit.
- A file of your home that stow does not own is never replaced: that package is skipped with a message.
- A package dropped from `stow.packages` is unlinked before anything else, so a starter module can take its files back. Delete a package directory only after `stow -D <package>`.
- A file both in your dotfiles and written by a starter module stops the switch before anything changes, and names the option to turn off: `programs.git.enable = false` to keep your own git config, `programs.starship.enable = false` for `starship.toml`. Your `~/.config/nvim` works as is, since the starter does not manage it.
- zsh reads `~/.config/zsh`, so a stowed `~/.zshrc` is ignored until you set `programs.zsh.enable = false`. zsh stays your login shell; call `compinit` in your own `.zshrc`, since the system one is off.

`home-stow` links both layouts and checks that a second run changes nothing, a removed file or dropped package loses its links, a foreign file is left alone and a conflict with home-manager stops the switch.

## Start `/home` empty at every boot

A host with a `disk` that adds `wipe-home` to its features forgets `/home` like `/`: every boot starts from an empty home, and only the paths someone declared come back. Clutter cannot pile up, and what your machine needs is written down in your config.

```nix
features = [ "wipe-home" ];
```

At every boot, the initrd moves the previous `@home` to `/.snapshots/home-wiped.<UTC date>`, read-only, and creates an empty one. A wiped home stays there for 7 days, then a later boot deletes it (the 2 most recent always stay): a file you forgot to declare can still be copied back. The dates come from the hardware clock, which every UEFI machine has. btrbk stops snapshotting `/home` and keeps snapshotting `/persist`, where declared paths live ([services.md](services.md#snapshots-btrbk)). home-manager links its files again at every boot, so your config, shell and tools are there as usual.

Declared paths live in `/persist/home/<you>` and are bind-mounted into your home at boot. The starter's own modules declare their state, only when `wipe-home` is on: zsh history (`~/.local/state/zsh`), zoxide (`~/.local/share/zoxide`), neovim plugins and undo history (`~/.local/share/nvim`, `~/.local/state/nvim`), the direnv allow list (`~/.local/share/direnv`), rootless Docker (`~/.local/share/docker`), the `stow` clone (`~/dotfiles`) and its state, the audio volumes and devices (`~/.local/state/wireplumber`, with `audio`), and the last run of user timers (`~/.local/share/systemd/timers`, so a weekly job missed while the machine was off still catches up). Declare yours in your user's home config, relative to your home:

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

Prefer directories to files: an application that saves by writing a new file and renaming it over the old one fails on a persisted file (it is a bind mount, which a rename cannot replace). `mode` only applies when the directory is first created in `/persist`. Without `wipe-home` these lines do nothing, so the same user works on hosts that keep their home.

### List what your home needs to keep

On the machine you are moving from, see where your files are and which ones changed recently; caches and anything your config rebuilds can go:

```sh
du -xh --max-depth=2 ~ 2>/dev/null | sort -h | tail -40
find ~ -xdev -type f -mtime -30 -not -path '*/.cache/*' -printf '%P\n' | cut -d/ -f1-2 | sort | uniq -c | sort -rn | head -50
```

Usual candidates: your projects and documents, `~/.ssh` and `~/.gnupg`, browser profiles (`~/.mozilla`, `~/.config/chromium`), password stores and keyrings (`~/.local/share/keyrings`), application sessions under `~/.config` and `~/.local/share`, and `~/.local/share/nix` for the flake settings you trusted. Packages installed with `nix profile add` are lost at reboot: list them in your home config instead. `~/.cache` starts empty, so the first launch of some tools is slower, and so is the first `nh os switch` after a boot, which rebuilds Nix's evaluation cache.

Once `wipe-home` is on, the last boot's snapshot shows what was wiped and is not declared yet:

```sh
ls /.snapshots
find /.snapshots/home-wiped.<date>/<you> -xdev -type f -printf '%P\n' | cut -d/ -f1-2 | sort | uniq -c | sort -rn
```

### Turn it on for an existing home

Declare your paths first, then apply the change at the next boot rather than live, so the empty persisted directories are not mounted over your current files: `nh os boot` (or `nixos-rebuild boot`), then reboot. Your old home is now in `/.snapshots/home-wiped.<date>`. Log in on a console, before a browser or another app writes fresh state, and copy each declared path back within 7 days, the starter's own included; it lands in `/persist`:

```sh
systemctl --user stop docker
old=/.snapshots/home-wiped.<date>/$USER
for path in projects dotfiles .local/state/zsh .local/share/zoxide .local/share/direnv .local/share/docker; do
  [ -e "$old/$path" ] && cp -a --reflink=auto "$old/$path/." ~/"$path/"
done
sudo systemctl restart home-manager-$USER
```

Restarting home-manager links your dotfiles, which `stow` found missing at boot. Never create `/persist/home/<you>` by hand as root: impermanence copies its owner onto your home directory.

`<host>-wipe-home` installs the host with `wipe-home`, reboots, and checks the home starts empty while a declared path survives ([ci.md](ci.md)).
