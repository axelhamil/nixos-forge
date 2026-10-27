# One person, one folder. A host gives them an account with `users.alice`
# (modules/hosts/example), and `nix run .#init` writes your own person from this file.
{
  users.alice = {
    fullName = "Alice Example";

    email = "alice@example.org";

    # Public SSH keys allowed to log in as alice.
    sshKeys = [ ];

    apps = [
      "zsh" # zsh with suggestions
      "git" # git, the person's name and email are the author
    ];

    # Any other file of modules/users/alice may set part of it.
    # module = { pkgs, ... }: { home.packages = [ pkgs.gimp ]; };
  };
}
