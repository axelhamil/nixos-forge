# One person, one folder: who they are and what follows them on every machine. Read it top
# to bottom: uncomment a line to use it. A host gives them an account with `users.alice`
# (modules/hosts/example), `nix run .#init` writes your own person from this file, and every
# field is described in docs/reference/options.md.
{
  users.alice = {
    # Full name: the description of the account, and the git author name when an email is set.
    fullName = "Alice Example";

    # Git author email; comment it out and git keeps no default author.
    email = "alice@example.org";

    # Public SSH keys allowed to log in as alice, on every machine where this person has an account.
    sshKeys = [ ]; # such as "ssh-ed25519 AAAA... alice@laptop"

    # Security keys that log alice in and run sudo without a password on hosts with "fido2",
    # one line of `pamu2fcfg -n -o pam://nixos-forge -i pam://nixos-forge` each (docs/how-to/fido2.md).
    # u2fKeys = [ ":owBYz...,pSJ7...,es256,+presence" ];

    # Apps that follow this person on every machine (none by default); a host adds its own.
    # Each app stands alone: an app of modules/apps/<name>, else any nixpkgs package.
    apps = [
      "zsh" # zsh with suggestions, syntax highlighting and a shared history; the login shell of whoever lists it (docs/reference/apps.md)
      "starship" # the starship prompt in plain-text symbols (docs/reference/apps.md)
      "git" # git with the settings git's own developers use; the person's name and email are the author (docs/reference/apps.md)
      "delta" # delta, a pager for git diffs and blame (docs/reference/apps.md)
      "neovim" # neovim as $EDITOR, with the compiler, tree-sitter, ripgrep and fd most configs expect (docs/reference/apps.md)
      "fzf" # fuzzy finder on Ctrl+T, Ctrl+R, Alt+C, listed by fd so .gitignore is respected (docs/reference/apps.md)
      "zoxide" # `z` jumps to a directory you visited (docs/reference/apps.md)
      "eza" # file listings with git status, as ll, la, lt (docs/reference/apps.md)
      "bat" # bat, a `cat` with syntax highlighting (docs/reference/apps.md)
      "btop" # a resource monitor (docs/reference/apps.md)
      "ripgrep" # `rg`, a fast grep (nixpkgs)
      "fd" # a fast find (nixpkgs)
      "jq" # a JSON processor (nixpkgs)
      # "stow" # links your dotfiles repository, ~/dotfiles by default, into your home at each switch (docs/reference/apps.md)
    ];

    # Own home-manager config, on every machine: packages, git aliases, editor settings.
    # Any other file of modules/users/alice may set part of it (docs/reference/apps.md).
    # module = { pkgs, ... }: { home.packages = [ pkgs.gimp ]; };
  };
}
