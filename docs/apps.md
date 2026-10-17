# Apps: your config in Nix, without dotfiles

Everything a user runs is an app: the desktop, the shell, a browser. Its program and its settings live in one folder, `modules/apps/<name>`, written in Nix, so a new machine comes up with your config and there is no dotfiles repository to clone.

## What an app is

One folder that publishes up to two parts:

- `apps.<name>.nixos`: the system side, such as a PAM service, a login shell or fonts.
- `apps.<name>.homeManager`: the user side, through [home-manager](https://nix-community.github.io/home-manager/): the program and its settings.

A host names it in `apps` (every user of the machine gets it) or in `users.<you>.apps` (you only). An app that brings others imports them with `helpers.appPart`, so an app reached twice, such as `shell` and `zsh`, is imported once.

```nix
apps = [ "hyprland" "direnv" "firefox" ];
users.alice.apps = [ "shell" "stow" ];
```

## Starter apps

| App | What it brings |
|---|---|
| `shell` | every user's by default: `zsh`, `starship`, `git`, `neovim`, `cli` |
| `hyprland` | the desktop, with `greetd`, `waybar`, `fuzzel`, `kitty`, `mako` and `lock` ([desktop.md](desktop.md)) |
| `direnv` | per-project dev shells with nix-direnv ([dev.md](dev.md)) |
| `firefox` | Firefox with telemetry off, its profile kept |
| `stow` | links your dotfiles repository, for those who keep one ([below](#keep-your-dotfiles-with-stow)) |

Each brought app also works alone: list `zsh` and `git` instead of `shell` to leave the rest out.

## Name any nixpkgs package

A name with no folder in `modules/apps` installs the nixpkgs package of that name: `"vlc"`, `"obsidian"`, or an attribute path such as `"kdePackages.kate"`. Unfree packages named there are allowed. A name that is neither stops the build and lists the starter's apps.

## Add an app

Create `modules/apps/<name>/default.nix`, `git add` it, and name it in `apps`:

```nix
{
  apps.<name>.homeManager = {
    programs.<name>.enable = true;
  };
}
```

A `check.nix` with `config.helpers.appCheck "<name>" { testScripts = [ ./test.py ]; }` boots it in a VM test, `app-<name>`. [`modules/apps/firefox`](../modules/apps/firefox/default.nix) is the model.

## Change a starter default

Every setting an app writes goes through `helpers.mkDefaults`: set the same option yourself and yours wins, with no `mkForce`. Its `enable` flags do not: to drop an app, remove it from `apps`. Put your settings in an app of your own, `modules/users/<you>/default.nix`, and list it next to `shell`:

```nix
{
  apps.<you>.homeManager = {
    programs.starship.presets = [ ];
    programs.git.settings.user = {
      name = "Your Name";
      email = "you@example.org";
    };
    programs.kitty.settings.font_size = 12;
  };
}
```

```nix
users.<you>.apps = [ "shell" "<you>" ];
```

## The shell

- **zsh** with autosuggestions, syntax highlighting and a shared, deduplicated history of 100,000 lines. Its NixOS part makes it the login shell of a user whose apps (or the host's) contain `zsh` or `shell`, completes system commands and skips the global `compinit`, so completion is initialised once, by home-manager. That switch is system-wide: another zsh user of the host needs home-manager's zsh too, or gets no completion.
- **starship** prompt with plain text symbols, readable on a console without a Nerd Font. Once you have one: `programs.starship.presets = [ ];`.
- **git** with the settings git's own developers use: `main` as first branch, recent branches first, `push.autoSetupRemote`, pruned fetches, histogram diffs with moved lines highlighted, the diff shown while writing a commit, `zdiff3` conflicts, `rerere`, rebases that autostash and update stacked branches, autocorrect on prompt. Diffs and blame page through [delta](https://github.com/dandavison/delta). The host's `fullName` and `email` are the author. To sign with your SSH key:

  ```nix
  programs.git.signing = {
    format = "ssh";
    key = "~/.ssh/id_ed25519.pub";
    signByDefault = true;
  };
  ```

- **neovim** as `$EDITOR` (`vi` and `vim` too), with the compiler, tree-sitter, ripgrep and fd most configurations expect. Its config is yours, in `~/.config/nvim` ([kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim), [LazyVim](https://www.lazyvim.org)). Language servers a plugin downloads (Mason) need the `nix-ld` feature ([dev.md](dev.md)).
- **cli**: fzf (`Ctrl-R` history, `Ctrl-T` files, `Alt-C` directories, listed by fd so `.gitignore` is respected), zoxide (`z <part of a path>`), eza with git status (`ll`, `la`, `lt`), bat, ripgrep, fd, btop and jq.

Files follow the XDG layout: zsh in `~/.config/zsh`, its history in `~/.local/state/zsh`, Nix profiles in `~/.local/state/nix`. The only file left in your home is `~/.zshenv`, which tells zsh where its config is.

## Keep your dotfiles with stow

`stow` is optional, for a dotfiles repository you keep. Add it next to `shell`, clone the repository to `~/dotfiles`, and every switch links it into your home with [GNU stow](https://www.gnu.org/software/stow/). Edits in the repository are live; a new file is linked at the next switch, or right away with `systemctl restart home-manager-$USER`.

```nix
users.<you>.apps = [ "shell" "stow" ];
```

Both common layouts work with no setting:

- **A mirror of your home**: `~/dotfiles/.zshrc`, `~/dotfiles/.config/nvim/init.lua`. Recognised when the repository root holds dot files; its `.git`, `README` and `LICENSE` are not linked.
- **Stow packages**: one directory per tool, `~/dotfiles/nvim/.config/nvim/init.lua`. Every top-level directory is linked, or only those you list.

Options: `stow.directory` (`~/dotfiles` by default), `stow.packages` (such as `[ "nvim" "tmux" ]`, empty for everything), `stow.dotfiles` (stow's `--dotfiles`, off by default).

What to expect:

- Directories stay real, only files are linked (`--no-folding`), so home-manager and stow share `~/.config`.
- Without the repository nothing happens: a fresh install boots, then you clone and restart the unit.
- A file of your home that stow does not own is never replaced: that package is skipped with a message.
- A package dropped from `stow.packages` is unlinked first, so a starter app can take its files back. Delete a package directory only after `stow -D <package>`.
- A file both in your dotfiles and written by an app stops the switch before anything changes, and names the option to turn off: `programs.git.enable = false` to keep your own git config, `programs.starship.enable = false` for `starship.toml`.
- zsh reads `~/.config/zsh`, so a stowed `~/.zshrc` is ignored until `programs.zsh.enable = false`. zsh stays your login shell; call `compinit` in your own `.zshrc`, since the system one is off.

To start `/home` empty at every boot and keep only declared paths, see [wipe-home.md](wipe-home.md).
