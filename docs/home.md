# Shell and tools

The `shell` bundle gives a user a ready shell with no choice to make: zsh, starship, git, neovim and a set of CLI tools, all through home-manager. Each tool is its own module under `modules/home/`, so you can drop or swap one.

## Use it

The example user already has it. For another user, in `modules/users/<you>/default.nix`:

```nix
{ config, ... }:

let
  inherit (config.flake.modules) nixos homeManager;
in
{
  flake.modules.nixos.<you> = { pkgs, ... }: {
    imports = [ nixos.zsh ];
    users.users.<you>.shell = pkgs.zsh;
  };

  flake.modules.homeManager.<you> = {
    imports = [ homeManager.shell ];
  };
}
```

To pick tools one by one, import them instead of `shell`: `homeManager.zsh`, `starship`, `git`, `neovim`, `cli`. Every value is a default: set the same option in your user's home config to change it, `false` included.

## What is in it

- **zsh** with autosuggestions, syntax highlighting and a shared, deduplicated history of 100,000 lines. The NixOS `zsh` module makes it a login shell, completes system commands and skips the global `compinit`, so completion is initialised once, by home-manager, instead of twice. That switch is system-wide: another zsh user of the host needs home-manager's zsh too (import `homeManager.zsh`), or gets no completion.
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

Import `homeManager.stow` next to `shell`, clone your dotfiles repository to `~/dotfiles`, and every switch links it into your home with [GNU stow](https://www.gnu.org/software/stow/). Edits in the repository are live; a new file is linked at the next switch, or right away with `systemctl restart home-manager-$USER`.

```nix
flake.modules.homeManager.<you> = {
  imports = [
    homeManager.shell
    homeManager.stow
  ];
};
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
