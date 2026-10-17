# One person, one folder: who they are and what follows them on every machine. Read it top
# to bottom: uncomment a line to use it. A host gives them an account with `users.alice`
# (modules/hosts/example), `nix run .#init` writes your own person from this file, and every
# field is described in docs/options.md.
{
  users.alice = {
    # Full name, shown by the greeter and used as git author name.
    fullName = "Alice Example";

    # Email used as git author email; comment it out for none.
    email = "alice@example.org";

    # Public SSH keys allowed to log in as alice, on every machine where this person has an account.
    sshKeys = [ ]; # such as "ssh-ed25519 AAAA... alice@laptop"

    # Apps that follow this person on every machine, "shell" by default: zsh, starship, git, neovim
    # and CLI tools. "stow" links your dotfiles (docs/apps.md); a host adds its own.
    # apps = [ "shell" "stow" "obsidian" ];

    # Own home-manager config, on every machine: packages, git aliases, editor settings.
    # Any other file of modules/users/alice may set part of it (docs/apps.md).
    # module = { pkgs, ... }: { home.packages = [ pkgs.gimp ]; };
  };
}
