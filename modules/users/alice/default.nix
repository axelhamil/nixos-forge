# One person, one folder: who they are and what follows them on every machine. Read it top
# to bottom: uncomment a line to use it. A host gives them an account with `users.alice`
# (modules/hosts/example), `nix run .#init` writes your own person from this file, and every
# field is described in docs/options.md.
{
  users.alice = {
    # Full name: the description of the account, and the git author name when an email is set.
    fullName = "Alice Example";

    # Git author email; comment it out and git keeps no default author.
    email = "alice@example.org";

    # Public SSH keys allowed to log in as alice, on every machine where this person has an account.
    sshKeys = [ ]; # such as "ssh-ed25519 AAAA... alice@laptop"

    # Apps that follow this person on every machine (none by default); a host adds its own.
    # Each app stands alone: an app of modules/apps/<name>, else any nixpkgs package.
    apps = [
      "zsh" # zsh with suggestions, syntax highlighting and a shared history; the login shell of whoever lists it (docs/apps.md)
      "starship" # the starship prompt in plain-text symbols (docs/apps.md)
      "git" # git with the settings git's own developers use; the person's name and email are the author (docs/apps.md)
      "delta" # delta, a pager for git diffs and blame (docs/apps.md)
      "neovim" # neovim as $EDITOR, with the compiler, tree-sitter, ripgrep and fd most configs expect (docs/apps.md)
      "fzf" # fuzzy finder on Ctrl+T, Ctrl+R, Alt+C, listed by fd so .gitignore is respected (docs/apps.md)
      "zoxide" # `z` jumps to a directory you visited (docs/apps.md)
      "eza" # file listings with git status, as ll, la, lt (docs/apps.md)
      "bat" # bat, a `cat` with syntax highlighting (docs/apps.md)
      "btop" # a resource monitor (docs/apps.md)
      "ripgrep" # `rg`, a fast grep (nixpkgs)
      "fd" # a fast find (nixpkgs)
      "jq" # a JSON processor (nixpkgs)
      # "stow" # links your dotfiles repository, ~/dotfiles by default, into your home at each switch (docs/apps.md)
    ];

    # Own home-manager config, on every machine: packages, git aliases, editor settings.
    # Any other file of modules/users/alice may set part of it (docs/apps.md).
    # module = { pkgs, ... }: { home.packages = [ pkgs.gimp ]; };
  };
}
