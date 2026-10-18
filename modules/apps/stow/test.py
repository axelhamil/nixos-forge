import shlex


def as_user(user, command):
    return machine.succeed(f"runuser -u {user} -- sh -c {shlex.quote(command)}")


def as_tester(command):
    return as_user("tester", command)


def restart(user):
    return f"systemctl reset-failed home-manager-{user}.service; systemctl restart home-manager-{user}.service"


def activate(user="tester"):
    machine.succeed(restart(user))


def logged(text, user="tester"):
    machine.succeed(f"journalctl -u home-manager-{user}.service | grep -qF {shlex.quote(text)}")


machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("home-manager-tester.service")
logged("no dotfiles in /home/tester/dotfiles yet")

as_tester("mkdir -p ~/dotfiles/nvim/.config/nvim/lua ~/dotfiles/zsh")
as_tester("echo 'vim.g.from_dotfiles = 1' > ~/dotfiles/nvim/.config/nvim/init.lua")
as_tester("echo 'return {}' > ~/dotfiles/nvim/.config/nvim/lua/extra.lua")
as_tester("echo 'alias from_dotfiles=true' > ~/dotfiles/zsh/.zprofile")
activate()

assert as_tester("readlink -f ~/.config/nvim/init.lua").strip() == "/home/tester/dotfiles/nvim/.config/nvim/init.lua"
as_tester("test -d ~/.config/nvim && test ! -L ~/.config/nvim")
as_tester("test -L ~/.zprofile")

activate()
as_tester("test -L ~/.config/nvim/init.lua")

as_tester("rm ~/dotfiles/nvim/.config/nvim/lua/extra.lua")
activate()
as_tester("test ! -e ~/.config/nvim/lua/extra.lua")

as_tester("echo stray > ~/.stray && mkdir -p ~/dotfiles/stray && echo dotfiles > ~/dotfiles/stray/.stray")
activate()
assert as_tester("cat ~/.stray").strip() == "stray", "stow replaced a file it does not own"
logged("stray not linked")
as_tester("rm -r ~/dotfiles/stray ~/.stray")

as_tester("mkdir -p ~/dotfiles/git/.config/git && echo '[user]' > ~/dotfiles/git/.config/git/config")
machine.fail(restart("tester"))
logged("programs.git.enable = false")
as_tester("rm -r ~/dotfiles/git")
activate()

machine.succeed("/run/current-system/specialisation/fewer/bin/switch-to-configuration test")
machine.wait_for_unit("home-manager-tester.service")
as_tester("test ! -e ~/.config/nvim/init.lua")
as_tester("test -L ~/.zprofile")

as_user("mirror", "mkdir -p ~/dotfiles/.config/tool ~/dotfiles/.git")
as_user("mirror", "echo setting > ~/dotfiles/.config/tool/config && echo docs > ~/dotfiles/README.md && echo ref > ~/dotfiles/.git/HEAD")
activate("mirror")
assert as_user("mirror", "readlink -f ~/.config/tool/config").strip() == "/home/mirror/dotfiles/.config/tool/config"
as_user("mirror", "test ! -e ~/README.md && test ! -e ~/.git")

as_tester("nix profile list")
as_tester("test ! -e ~/.nix-profile && test ! -e ~/.nix-defexpr && test -d ~/.local/state/nix")
