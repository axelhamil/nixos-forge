# Apps: your config in Nix, without dotfiles

Everything a user runs is an app: the desktop, the shell, a browser. Its program and its settings live in one folder, `modules/apps/<name>`, written in Nix, so a new machine comes up with your config and there is no dotfiles repository to clone.

This page covers what an app is, the ones upstream ships, how to add yours and how to bend a shipped default your way.

## What an app is

One folder that publishes up to two parts:

- `apps.<name>.nixos`: the system side, such as a PAM service, a login shell or fonts.
- `apps.<name>.homeManager`: the user side, through [home-manager](https://nix-community.github.io/home-manager/): the program and its settings.

Three places name apps:

- a host's `apps`: every user of that machine gets it;
- your person's `apps`, in `modules/users/<you>/default.nix`: it follows you on every machine (none by default; the example person lists the shell tools);
- a host's `users.<you>.apps`: you get it on that machine only, on top of your person's.

Wherever it is named, an app's NixOS part is on for the whole machine; only its home-manager part is per user.

```nix
apps = [ "hyprland" "waybar" "fuzzel" "kitty" "mako" "lock" "firefox" ]; # in modules/hosts/<host>
users.<you>.apps = [ "zsh" "starship" "git" "neovim" "ripgrep" "stow" ]; # in modules/users/<you>
```

Three rules keep apps honest:

- **No app brings another.** Each one works alone, and the list is the whole choice.
- **Neighbours are read, never imported.** An app adapts to another by reading its home-manager option (`config.programs.fuzzel.enable`): Hyprland binds `Super + D` only when `fuzzel` is listed.
- **Named twice, imported once.** An app listed by both the host and the person is imported a single time.

## Starter apps

| App | What it brings |
|---|---|
| `hyprland` | the Hyprland window manager with uwsm, its portal and a polkit agent ([desktop.md](desktop.md)) |
| `waybar`, `fuzzel`, `kitty`, `mako`, `lock` | a bar, a launcher, a terminal, notifications, a lock screen ([desktop.md](desktop.md)) |
| `upgrade-notify` | a desktop notification after each `auto-upgrade` ([updates.md](../explanation/updates.md#know-what-it-did)) |
| `direnv` | per-project dev shells with nix-direnv ([dev.md](dev.md)) |
| `firefox` | Firefox with telemetry off, its profile kept |
| `zsh`, `starship`, `git`, `delta`, `neovim` | the shell, its prompt, git, its diff pager, the editor ([below](#the-shell)) |
| `fzf`, `zoxide`, `eza`, `bat`, `btop` | command line tools with their config ([below](#the-shell)) |
| `steam`, `gamemode`, `mangohud` | Steam with Proton-GE, Feral GameMode, and an FPS overlay that logs runs ([specialisations.md](../how-to/specialisations.md#games)) |
| `nix-index` | `nix-locate` on a prebuilt database, and `, <command>` to run a command without installing it ([dev.md](dev.md#nix-index-opt-in-app)) |
| `gh` | the GitHub CLI, its login kept with `wipe-home` |
| `stow` | links your dotfiles repository, for those who keep one ([below](#keep-your-dotfiles-with-stow)) |

Tools with nothing to configure, such as `ripgrep`, `fd` or `jq`, are plain nixpkgs names.

## Name any nixpkgs package

A name with no folder in `modules/apps` installs the nixpkgs package of that name: `"vlc"`, `"obsidian"`, or an attribute path such as `"kdePackages.kate"`. Unfree packages named there are allowed. A name that is neither stops the build and lists the configured apps.

## Add an app

Create `modules/apps/<name>/default.nix`, `git add` it, and name it in `apps`:

```nix
{
  apps.<name>.homeManager = {
    programs.<name>.enable = true;
  };
}
```

A `check.nix` with `config.helpers.appCheck "<name>" { testScripts = [ ./test.py ]; }` boots it in a VM test, `app-<name>`. [`modules/apps/firefox`](../../modules/apps/firefox/default.nix) is the model.

## Change a starter default

Every setting an app writes goes through `helpers.mkDefaults`: set the same option yourself and yours wins, with no `mkForce`. Its `enable` flags do not: to drop an app, remove it from `apps`. Your own settings go in your person's `module`, the home-manager config that follows you on every machine that gives you an account:

```nix
{
  users.<you>.module = {
    programs.starship.presets = [ ];
    programs.git.settings.alias.st = "status";
    programs.kitty.settings.font_size = 12;
  };
}
```

It grows with you: any other file of `modules/users/<you>`, such as `git.nix` or `neovim.nix`, may set its own part of `users.<you>.module`, and the parts merge. Settings you want on several people, or on a host's users, belong in an app instead.

## The shell

- **zsh** with autosuggestions, syntax highlighting and a shared, deduplicated history of 100,000 lines. The registry makes it the login shell of a user whose apps (or the host's) name `zsh`. Its NixOS part completes system commands and skips the global `compinit`, so completion is initialised once, by home-manager. That switch is system-wide: another zsh user of the host needs home-manager's zsh too, or gets no completion.
- **starship** prompt with plain text symbols, readable on a console without a Nerd Font. Once you have one: `programs.starship.presets = [ ];`.
- **git** with the settings git's own developers use: `main` as first branch, recent branches first, `push.autoSetupRemote`, pruned fetches, histogram diffs with moved lines highlighted, the diff shown while writing a commit, `zdiff3` conflicts, `rerere`, rebases that autostash and update stacked branches, autocorrect on prompt. With `delta`, diffs and blame page through [delta](https://github.com/dandavison/delta). Your person's `fullName` and `email` are the author. To sign with your SSH key:

  ```nix
  programs.git.signing = {
    format = "ssh";
    key = "~/.ssh/id_ed25519.pub";
    signByDefault = true;
  };
  ```

- **neovim** as `$EDITOR` (`vi` and `vim` too), with the compiler, tree-sitter, ripgrep and fd most configurations expect. Its config is yours, in `~/.config/nvim` ([kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim), [LazyVim](https://www.lazyvim.org)). Language servers a plugin downloads (Mason) need the `nix-ld` feature ([dev.md](dev.md)).
- **fzf**: `Ctrl-R` history, `Ctrl-T` files, `Alt-C` directories, listed by its own fd so `.gitignore` is respected.
- **zoxide**: `z <part of a path>`, its database kept.
- **eza** with git status as `ll`, `la`, `lt`, leaving `ls` alone.
- **bat** and **btop** with their defaults.

The shell integrations of fzf, zoxide and starship turn on when `zsh` is listed.

Files follow the XDG layout: zsh in `~/.config/zsh`, its history in `~/.local/state/zsh`, Nix profiles in `~/.local/state/nix`. The only file left in your home is `~/.zshenv`, which tells zsh where its config is.

## Keep your dotfiles with stow

`stow` is optional, for a dotfiles repository you keep. Add it to your person's apps, clone the repository to `~/dotfiles`, and every switch links it into your home with [GNU stow](https://www.gnu.org/software/stow/). Edits in the repository are live; a new file is linked at the next switch, or right away with `systemctl restart home-manager-$USER`.

```nix
users.<you>.apps = [ "zsh" "git" "neovim" "stow" ];
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

To start `/home` empty at every boot and keep only declared paths, see [wipe-home.md](../how-to/wipe-home.md).
